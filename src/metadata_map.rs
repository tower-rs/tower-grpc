use http;
use metadata_key::InvalidMetadataKey;
use metadata_key::MetadataKey;
use metadata_value::MetadataValue;

pub use self::as_metadata_key::AsMetadataKey;
pub use self::into_metadata_key::IntoMetadataKey;

/// A set of gRPC custom metadata entries.
///
/// # Examples
///
/// Basic usage
///
/// ```
/// # use tower_grpc::metadata::*;
/// let mut map = MetadataMap::new();
///
/// map.insert("x-host", "example.com".parse().unwrap());
/// map.insert("x-number", "123".parse().unwrap());
///
/// assert!(map.contains_key("x-host"));
/// assert!(!map.contains_key("x-location"));
///
/// assert_eq!(map.get("x-host").unwrap(), "example.com");
///
/// map.remove("x-host");
///
/// assert!(!map.contains_key("x-host"));
/// ```
#[derive(Clone, Debug, Default)]
pub struct MetadataMap {
    headers: http::HeaderMap<MetadataValue>,
}

/// `HeaderMap` entry iterator.
///
/// Yields `(&HeaderName, &value)` tuples. The same header name may be yielded
/// more than once if it has more than one associated value.
#[derive(Debug)]
pub struct Iter<'a, T: 'a> {
    inner: http::header::Iter<'a, T>,
}

/// A drain iterator of all values associated with a single metadata key.
#[derive(Debug)]
pub struct ValueDrain<'a, T: 'a> {
    inner: http::header::ValueDrain<'a, T>,
}

/// A drain iterator for `MetadataMap`.
#[derive(Debug)]
pub struct Drain<'a, T: 'a> {
    inner: http::header::Drain<'a, T>,
}

/// An iterator over `MetadataMap` keys.
///
/// Each header name is yielded only once, even if it has more than one
/// associated value.
#[derive(Debug)]
pub struct Keys<'a, T: 'a> {
    inner: http::header::Keys<'a, T>,
}

/// `MetadataMap` value iterator.
///
/// Each value contained in the `MetadataMap` will be yielded.
#[derive(Debug)]
pub struct Values<'a, T: 'a> {
    inner: http::header::Values<'a, T>,
}

/// An iterator of all values associated with a single metadata key.
#[derive(Debug)]
pub struct ValueIter<'a, T: 'a> {
    inner: http::header::ValueIter<'a, T>,
}

/// An iterator of all values associated with a single metadata key.
#[derive(Debug)]
pub struct ValueIterMut<'a, T: 'a> {
    inner: http::header::ValueIterMut<'a, T>,
}

/// A view to all values stored in a single entry.
///
/// This struct is returned by `MetadataMap::get_all`.
#[derive(Debug)]
pub struct GetAll<'a> {
    inner: http::header::GetAll<'a, MetadataValue>
}

/// A view into a single location in a `MetadataMap`, which may be vacant or occupied.
#[derive(Debug)]
pub enum Entry<'a, T: 'a> {
    /// An occupied entry
    Occupied(OccupiedEntry<'a, T>),

    /// A vacant entry
    Vacant(VacantEntry<'a, T>),
}

/// A view into a single empty location in a `MetadataMap`.
///
/// This struct is returned as part of the `Entry` enum.
#[derive(Debug)]
pub struct VacantEntry<'a, T: 'a> {
    inner: http::header::VacantEntry<'a, T>,
}

/// A view into a single occupied location in a `MetadataMap`.
///
/// This struct is returned as part of the `Entry` enum.
#[derive(Debug)]
pub struct OccupiedEntry<'a, T: 'a> {
    inner: http::header::OccupiedEntry<'a, T>,
}

// ===== impl MetadataMap =====

impl MetadataMap {
    /// Create an empty `MetadataMap`.
    ///
    /// The map will be created without any capacity. This function will not
    /// allocate.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let map = MetadataMap::new();
    ///
    /// assert!(map.is_empty());
    /// assert_eq!(0, map.capacity());
    /// ```
    pub fn new() -> Self {
        MetadataMap::with_capacity(0)
    }

    /// Convert an HTTP HeaderMap to a MetadataMap
    pub fn from_headers(mut headers: http::HeaderMap) -> Self {
        let mut map = Self::with_capacity(headers.len());
        for (name, values) in headers.drain() {
            let key = MetadataKey { inner: name };
            for value in values {
                map.append(&key, MetadataValue { inner: value });
            }
        }
        map
    }

    /// Convert a MetadataMap into a HTTP HeaderMap
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::new();
    /// map.insert("x-host", "example.com".parse().unwrap());
    ///
    /// let http_map = map.into_headers();
    ///
    /// assert_eq!(http_map.get("x-host").unwrap(), "example.com");
    /// ```
    pub fn into_headers(mut self) -> http::HeaderMap {
        let mut map = http::HeaderMap::with_capacity(self.len());
        for (key, values) in self.drain() {
            for value in values {
                map.append(&key.inner, value.inner);
            }
        }
        map
    }

    /// Create an empty `MetadataMap` with the specified capacity.
    ///
    /// The returned map will allocate internal storage in order to hold about
    /// `capacity` elements without reallocating. However, this is a "best
    /// effort" as there are usage patterns that could cause additional
    /// allocations before `capacity` metadata entries are stored in the map.
    ///
    /// More capacity than requested may be allocated.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let map: MetadataMap = MetadataMap::with_capacity(10);
    ///
    /// assert!(map.is_empty());
    /// assert!(map.capacity() >= 10);
    /// ```
    pub fn with_capacity(capacity: usize) -> MetadataMap {
        MetadataMap {
            headers: http::HeaderMap::with_capacity(capacity),
        }
    }

    /// Returns the number of metadata entries stored in the map.
    ///
    /// This number represents the total number of **values** stored in the map.
    /// This number can be greater than or equal to the number of **keys**
    /// stored given that a single key may have more than one associated value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::new();
    ///
    /// assert_eq!(0, map.len());
    ///
    /// map.insert("x-host-ip", "127.0.0.1".parse().unwrap());
    /// map.insert("x-host-name", "localhost".parse().unwrap());
    ///
    /// assert_eq!(2, map.len());
    ///
    /// map.append("x-mime-type", "text/html".parse().unwrap());
    ///
    /// assert_eq!(3, map.len());
    /// ```
    pub fn len(&self) -> usize {
        self.headers.len()
    }

    /// Returns the number of keys stored in the map.
    ///
    /// This number will be less than or equal to `len()` as each key may have
    /// more than one associated value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::new();
    ///
    /// assert_eq!(0, map.keys_len());
    ///
    /// map.insert("x-mime", "text/plain".parse().unwrap());
    /// map.insert("x-host", "localhost".parse().unwrap());
    ///
    /// assert_eq!(2, map.keys_len());
    ///
    /// map.insert("x-mime", "text/html".parse().unwrap());
    ///
    /// assert_eq!(2, map.keys_len());
    /// ```
    pub fn keys_len(&self) -> usize {
        self.headers.len()
    }

    /// Returns true if the map contains no elements.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::new();
    ///
    /// assert!(map.is_empty());
    ///
    /// map.insert("x-host", "hello.world".parse().unwrap());
    ///
    /// assert!(!map.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.headers.is_empty()
    }

    /// Clears the map, removing all key-value pairs. Keeps the allocated memory
    /// for reuse.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::new();
    /// map.insert("x-host", "hello.world".parse().unwrap());
    ///
    /// map.clear();
    /// assert!(map.is_empty());
    /// assert!(map.capacity() > 0);
    /// ```
    pub fn clear(&mut self) {
        self.headers.clear();
    }

    /// Returns the number of custom metadata entries the map can hold without
    /// reallocating.
    ///
    /// This number is an approximation as certain usage patterns could cause
    /// additional allocations before the returned capacity is filled.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::new();
    ///
    /// assert_eq!(0, map.capacity());
    ///
    /// map.insert("x-host", "hello.world".parse().unwrap());
    /// assert_eq!(6, map.capacity());
    /// ```
    pub fn capacity(&self) -> usize {
        self.headers.capacity()
    }

    /// Reserves capacity for at least `additional` more custom metadata to be
    /// inserted into the `MetadataMap`.
    ///
    /// The metadata map may reserve more space to avoid frequent reallocations.
    /// Like with `with_capacity`, this will be a "best effort" to avoid
    /// allocations until `additional` more custom metadata is inserted. Certain
    /// usage patterns could cause additional allocations before the number is
    /// reached.
    ///
    /// # Panics
    ///
    /// Panics if the new allocation size overflows `usize`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::new();
    /// map.reserve(10);
    /// # map.insert("x-host", "bar".parse().unwrap());
    /// ```
    pub fn reserve(&mut self, additional: usize) {
        self.headers.reserve(additional);
    }

    /// Returns a reference to the value associated with the key.
    ///
    /// If there are multiple values associated with the key, then the first one
    /// is returned. Use `get_all` to get all values associated with a given
    /// key. Returns `None` if there are no values associated with the key.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::new();
    /// assert!(map.get("x-host").is_none());
    ///
    /// map.insert("x-host", "hello".parse().unwrap());
    /// assert_eq!(map.get("x-host").unwrap(), &"hello");
    /// assert_eq!(map.get("x-host").unwrap(), &"hello");
    ///
    /// map.append("x-host", "world".parse().unwrap());
    /// assert_eq!(map.get("x-host").unwrap(), &"hello");
    /// ```
    pub fn get<K>(&self, key: K) -> Option<&MetadataValue>
        where K: AsMetadataKey
    {

        key.get(self)
    }

    /// Returns a mutable reference to the value associated with the key.
    ///
    /// If there are multiple values associated with the key, then the first one
    /// is returned. Use `entry` to get all values associated with a given
    /// key. Returns `None` if there are no values associated with the key.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::default();
    /// map.insert("x-host", "hello".parse().unwrap());
    /// map.get_mut("x-host").unwrap().set_sensitive(true);
    ///
    /// assert!(map.get("x-host").unwrap().is_sensitive());
    /// ```
    pub fn get_mut<K>(&mut self, key: K) -> Option<&mut MetadataValue>
        where K: AsMetadataKey
    {
        key.get_mut(self)
    }

    /// Returns a view of all values associated with a key.
    ///
    /// The returned view does not incur any allocations and allows iterating
    /// the values associated with the key.  See [`GetAll`] for more details.
    /// Returns `None` if there are no values associated with the key.
    ///
    /// [`GetAll`]: struct.GetAll.html
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::new();
    ///
    /// map.insert("x-host", "hello".parse().unwrap());
    /// map.append("x-host", "goodbye".parse().unwrap());
    ///
    /// let view = map.get_all("x-host");
    ///
    /// let mut iter = view.iter();
    /// assert_eq!(&"hello", iter.next().unwrap());
    /// assert_eq!(&"goodbye", iter.next().unwrap());
    /// assert!(iter.next().is_none());
    /// ```
    pub fn get_all<K>(&self, key: K) -> GetAll
        where K: AsMetadataKey
    {
        GetAll {
            inner: key.get_all(self),
        }
    }

    /// Returns true if the map contains a value for the specified key.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::new();
    /// assert!(!map.contains_key("x-host"));
    ///
    /// map.insert("x-host", "world".parse().unwrap());
    /// assert!(map.contains_key("x-host"));
    /// ```
    pub fn contains_key<K>(&self, key: K) -> bool
        where K: AsMetadataKey
    {
        key.contains_key(self)
    }

    /// An iterator visiting all key-value pairs.
    ///
    /// The iteration order is arbitrary, but consistent across platforms for
    /// the same crate version. Each key will be yielded once per associated
    /// value. So, if a key has 3 associated values, it will be yielded 3 times.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::new();
    ///
    /// map.insert("x-word", "hello".parse().unwrap());
    /// map.append("x-word", "goodbye".parse().unwrap());
    /// map.insert("x-number", "123".parse().unwrap());
    ///
    /// for (key, value) in map.iter() {
    ///     println!("{:?}: {:?}", key, value);
    /// }
    /// ```
    pub fn iter(&self) -> Iter<MetadataValue> {
        Iter { inner: self.headers.iter() }
    }

    /*
    /// An iterator visiting all key-value pairs, with mutable value references.
    ///
    /// The iterator order is arbitrary, but consistent across platforms for the
    /// same crate version. Each key will be yielded once per associated value,
    /// so if a key has 3 associated values, it will be yielded 3 times.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::default();
    ///
    /// map.insert("x-word", "hello".parse().unwrap());
    /// map.append("x-word", "goodbye".parse().unwrap());
    /// map.insert("x-number", "123".parse().unwrap());
    ///
    /// for (key, value) in map.iter_mut() {
    ///     value.set_sensitive(true);
    /// }
    /// ```
    // TODO(pgron): Expose this. Blocked by https://github.com/hyperium/http/pull/278
    pub fn iter_mut(&mut self) -> http::header::map::IterMut<MetadataValue> {
        self.headers.iter_mut()
    }*/

    /// An iterator visiting all keys.
    ///
    /// The iteration order is arbitrary, but consistent across platforms for
    /// the same crate version. Each key will be yielded only once even if it
    /// has multiple associated values.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::new();
    ///
    /// map.insert("x-word", "hello".parse().unwrap());
    /// map.append("x-word", "goodbye".parse().unwrap());
    /// map.insert("x-number", "123".parse().unwrap());
    ///
    /// for key in map.keys() {
    ///     println!("{:?}", key);
    /// }
    /// ```
    pub fn keys(&self) -> Keys<MetadataValue> {
        Keys { inner: self.headers.keys() }
    }

    /// An iterator visiting all values.
    ///
    /// The iteration order is arbitrary, but consistent across platforms for
    /// the same crate version.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::new();
    ///
    /// map.insert("x-word", "hello".parse().unwrap());
    /// map.append("x-word", "goodbye".parse().unwrap());
    /// map.insert("x-number", "123".parse().unwrap());
    ///
    /// for value in map.values() {
    ///     println!("{:?}", value);
    /// }
    /// ```
    pub fn values(&self) -> Values<MetadataValue> {
        Values { inner: self.headers.values() }
    }

/*
    /// An iterator visiting all values mutably.
    ///
    /// The iteration order is arbitrary, but consistent across platforms for
    /// the same crate version.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::default();
    ///
    /// map.insert("x-word", "hello".parse().unwrap());
    /// map.append("x-word", "goodbye".parse().unwrap());
    /// map.insert("x-number", "123".parse().unwrap());
    ///
    /// for value in map.values_mut() {
    ///     value.set_sensitive(true);
    /// }
    /// ```
    // TODO(pgron): Expose this. Blocked by https://github.com/hyperium/http/pull/278
    pub fn values_mut(&mut self) -> http::header::ValuesMut<MetadataValue> {
        self.headers.values_mut()
    }*/

    /// Clears the map, returning all entries as an iterator.
    ///
    /// The internal memory is kept for reuse.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::new();
    ///
    /// map.insert("x-word", "hello".parse().unwrap());
    /// map.append("x-word", "goodbye".parse().unwrap());
    /// map.insert("x-number", "123".parse().unwrap());
    ///
    /// let mut drain = map.drain();
    ///
    /// let (key, mut vals) = drain.next().unwrap();
    ///
    /// assert_eq!("x-word", key);
    /// assert_eq!("hello", vals.next().unwrap());
    /// assert_eq!("goodbye", vals.next().unwrap());
    /// assert!(vals.next().is_none());
    ///
    /// let (key, mut vals) = drain.next().unwrap();
    ///
    /// assert_eq!("x-number", key);
    /// assert_eq!("123", vals.next().unwrap());
    /// assert!(vals.next().is_none());
    /// ```
    pub fn drain(&mut self) -> Drain<MetadataValue> {
        Drain { inner: self.headers.drain() }
    }

    /// Gets the given key's corresponding entry in the map for in-place
    /// manipulation.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::default();
    ///
    /// let headers = &[
    ///     "content-length",
    ///     "x-hello",
    ///     "Content-Length",
    ///     "x-world",
    /// ];
    ///
    /// for &header in headers {
    ///     let counter = map.entry(header).unwrap().or_insert("".parse().unwrap());
    ///     *counter = format!("{}{}", counter.to_str().unwrap(), "1").parse().unwrap();
    /// }
    ///
    /// assert_eq!(map.get("content-length").unwrap(), "11");
    /// assert_eq!(map.get("x-hello").unwrap(), "1");
    /// ```
    pub fn entry<K>(&mut self, key: K) -> Result<Entry<MetadataValue>, InvalidMetadataKey>
        where K: AsMetadataKey
    {
        match key.entry(self) {
            Ok(entry) => {
                Ok(match entry {
                    http::header::Entry::Occupied(e) =>
                        Entry::Occupied(OccupiedEntry { inner: e }),
                    http::header::Entry::Vacant(e) =>
                        Entry::Vacant(VacantEntry { inner: e }),
                })
            }
            Err(_) => Err(InvalidMetadataKey::new()),
        }
    }

    /// Inserts a key-value pair into the map.
    ///
    /// If the map did not previously have this key present, then `None` is
    /// returned.
    ///
    /// If the map did have this key present, the new value is associated with
    /// the key and all previous values are removed. **Note** that only a single
    /// one of the previous values is returned. If there are multiple values
    /// that have been previously associated with the key, then the first one is
    /// returned. See `insert_mult` on `OccupiedEntry` for an API that returns
    /// all values.
    ///
    /// The key is not updated, though; this matters for types that can be `==`
    /// without being identical.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::new();
    /// assert!(map.insert("x-host", "world".parse().unwrap()).is_none());
    /// assert!(!map.is_empty());
    ///
    /// let mut prev = map.insert("x-host", "earth".parse().unwrap()).unwrap();
    /// assert_eq!("world", prev);
    /// ```
    pub fn insert<K>(&mut self, key: K, val: MetadataValue) -> Option<MetadataValue>
        where K: IntoMetadataKey
    {
        key.insert(self, val)
    }

    /// Inserts a key-value pair into the map.
    ///
    /// If the map did not previously have this key present, then `false` is
    /// returned.
    ///
    /// If the map did have this key present, the new value is pushed to the end
    /// of the list of values currently associated with the key. The key is not
    /// updated, though; this matters for types that can be `==` without being
    /// identical.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::new();
    /// assert!(map.insert("x-host", "world".parse().unwrap()).is_none());
    /// assert!(!map.is_empty());
    ///
    /// map.append("x-host", "earth".parse().unwrap());
    ///
    /// let values = map.get_all("x-host");
    /// let mut i = values.iter();
    /// assert_eq!("world", *i.next().unwrap());
    /// assert_eq!("earth", *i.next().unwrap());
    /// ```
    pub fn append<K>(&mut self, key: K, value: MetadataValue) -> bool
        where K: IntoMetadataKey
    {
        key.append(self, value)
    }

    /// Removes a key from the map, returning the value associated with the key.
    ///
    /// Returns `None` if the map does not contain the key. If there are
    /// multiple values associated with the key, then the first one is returned.
    /// See `remove_entry_mult` on `OccupiedEntry` for an API that yields all
    /// values.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::new();
    /// map.insert("x-host", "hello.world".parse().unwrap());
    ///
    /// let prev = map.remove("x-host").unwrap();
    /// assert_eq!("hello.world", prev);
    ///
    /// assert!(map.remove("x-host").is_none());
    /// ```
    pub fn remove<K>(&mut self, key: K) -> Option<MetadataValue>
        where K: AsMetadataKey
    {
        key.remove(self)
    }
}

// ===== impl Iter =====

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (&'a str, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|item| {
            let (ref name, ref value) = item;
            let item : Self::Item = (&name.as_str(), &value);
            item
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

unsafe impl<'a, T: Sync> Sync for Iter<'a, T> {}
unsafe impl<'a, T: Sync> Send for Iter<'a, T> {}

// ===== impl ValueDrain =====

impl<'a, T> Iterator for ValueDrain<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

unsafe impl<'a, T: Sync> Sync for ValueDrain<'a, T> {}
unsafe impl<'a, T: Send> Send for ValueDrain<'a, T> {}

// ===== impl Drain =====

impl<'a, T> Iterator for Drain<'a, T> {
    type Item = (MetadataKey, ValueDrain<'a, T>);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|item| {
            let (name, drain) = item;
            (MetadataKey { inner: name }, ValueDrain { inner: drain })
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

unsafe impl<'a, T: Sync> Sync for Drain<'a, T> {}
unsafe impl<'a, T: Send> Send for Drain<'a, T> {}

// ===== impl Keys =====

impl<'a, T> Iterator for Keys<'a, T> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|b| b.as_str())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a, T> ExactSizeIterator for Keys<'a, T> {}

// ===== impl Values ====

impl<'a, T> Iterator for Values<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

// ===== impl ValueIter =====

impl<'a, T: 'a> Iterator for ValueIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a, T: 'a> DoubleEndedIterator for ValueIter<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
    }
}

// ===== impl ValueIterMut =====

impl<'a, T: 'a> Iterator for ValueIterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<'a, T: 'a> DoubleEndedIterator for ValueIterMut<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
    }
}

unsafe impl<'a, T: Sync> Sync for ValueIterMut<'a, T> {}
unsafe impl<'a, T: Send> Send for ValueIterMut<'a, T> {}

// ===== impl Entry =====

impl<'a, T> Entry<'a, T> {
    /// Ensures a value is in the entry by inserting the default if empty.
    ///
    /// Returns a mutable reference to the **first** value in the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map: MetadataMap = MetadataMap::default();
    ///
    /// let keys = &[
    ///     "content-length",
    ///     "x-hello",
    ///     "Content-Length",
    ///     "x-world",
    /// ];
    ///
    /// for &key in keys {
    ///     let counter = map.entry(key)
    ///         .expect("valid key names")
    ///         .or_insert("".parse().unwrap());
    ///     *counter = format!("{}{}", counter.to_str().unwrap(), "1").parse().unwrap();
    /// }
    ///
    /// assert_eq!(map.get("content-length").unwrap(), "11");
    /// assert_eq!(map.get("x-hello").unwrap(), "1");
    /// ```
    pub fn or_insert(self, default: T) -> &'a mut T {
        use self::Entry::*;

        match self {
            Occupied(e) => e.into_mut(),
            Vacant(e) => e.insert(default),
        }
    }

    /// Ensures a value is in the entry by inserting the result of the default
    /// function if empty.
    ///
    /// The default function is not called if the entry exists in the map.
    /// Returns a mutable reference to the **first** value in the entry.
    ///
    /// # Examples
    ///
    /// Basic usage.
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::new();
    ///
    /// let res = map.entry("x-hello").unwrap()
    ///     .or_insert_with(|| "world".parse().unwrap());
    ///
    /// assert_eq!(res, "world");
    /// ```
    ///
    /// The default function is not called if the entry exists in the map.
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::new();
    /// map.insert("host", "world".parse().unwrap());
    ///
    /// let res = map.entry("host")
    ///     .expect("host is a valid string")
    ///     .or_insert_with(|| unreachable!());
    ///
    ///
    /// assert_eq!(res, "world");
    /// ```
    pub fn or_insert_with<F: FnOnce() -> T>(self, default: F) -> &'a mut T {
        use self::Entry::*;

        match self {
            Occupied(e) => e.into_mut(),
            Vacant(e) => e.insert(default()),
        }
    }

    /// Returns a reference to the entry's key
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::new();
    ///
    /// assert_eq!(map.entry("x-hello").unwrap().key(), "x-hello");
    /// ```
    pub fn key(&self) -> &str {
        use self::Entry::*;

        match *self {
            Vacant(ref e) => e.inner.key().as_str(),
            Occupied(ref e) => e.inner.key().as_str(),
        }
    }
}

// ===== impl VacantEntry =====

impl<'a, T> VacantEntry<'a, T> {
    /// Returns a reference to the entry's key
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::new();
    ///
    /// assert_eq!(map.entry("x-hello").unwrap().key(), "x-hello");
    /// ```
    pub fn key(&self) -> &str {
        self.inner.key().as_str()
    }

    /// Take ownership of the key
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::new();
    ///
    /// if let Entry::Vacant(v) = map.entry("x-hello").unwrap() {
    ///     assert_eq!(v.into_key().as_str(), "x-hello");
    /// }
    /// ```
    pub fn into_key(self) -> MetadataKey {
        MetadataKey { inner: self.inner.into_key() }
    }

    /// Insert the value into the entry.
    ///
    /// The value will be associated with this entry's key. A mutable reference
    /// to the inserted value will be returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::new();
    ///
    /// if let Entry::Vacant(v) = map.entry("x-hello").unwrap() {
    ///     v.insert("world".parse().unwrap());
    /// }
    ///
    /// assert_eq!(map.get("x-hello").unwrap(), "world");
    /// ```
    pub fn insert(self, value: T) -> &'a mut T {
        self.inner.insert(value)
    }

    /// Insert the value into the entry.
    ///
    /// The value will be associated with this entry's key. The new
    /// `OccupiedEntry` is returned, allowing for further manipulation.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::new();
    ///
    /// if let Entry::Vacant(v) = map.entry("x-hello").unwrap() {
    ///     let mut e = v.insert_entry("world".parse().unwrap());
    ///     e.insert("world2".parse().unwrap());
    /// }
    ///
    /// assert_eq!(map.get("x-hello").unwrap(), "world2");
    /// ```
    pub fn insert_entry(self, value: T) -> OccupiedEntry<'a, T> {
        OccupiedEntry {
            inner: self.inner.insert_entry(value)
        }
    }
}

// ===== impl OccupiedEntry =====

impl<'a, T> OccupiedEntry<'a, T> {
    /// Returns a reference to the entry's key.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::new();
    /// map.insert("host", "world".parse().unwrap());
    ///
    /// if let Entry::Occupied(e) = map.entry("host").unwrap() {
    ///     assert_eq!("host", e.key());
    /// }
    /// ```
    pub fn key(&self) -> &str {
        self.inner.key().as_str()
    }

    /// Get a reference to the first value in the entry.
    ///
    /// Values are stored in insertion order.
    ///
    /// # Panics
    ///
    /// `get` panics if there are no values associated with the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::new();
    /// map.insert("host", "hello.world".parse().unwrap());
    ///
    /// if let Entry::Occupied(mut e) = map.entry("host").unwrap() {
    ///     assert_eq!(e.get(), &"hello.world");
    ///
    ///     e.append("hello.earth".parse().unwrap());
    ///
    ///     assert_eq!(e.get(), &"hello.world");
    /// }
    /// ```
    pub fn get(&self) -> &T {
        self.inner.get()
    }

    /// Get a mutable reference to the first value in the entry.
    ///
    /// Values are stored in insertion order.
    ///
    /// # Panics
    ///
    /// `get_mut` panics if there are no values associated with the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::default();
    /// map.insert("host", "hello.world".parse().unwrap());
    ///
    /// if let Entry::Occupied(mut e) = map.entry("host").unwrap() {
    ///     e.get_mut().set_sensitive(true);
    ///     assert_eq!(e.get(), &"hello.world");
    ///     assert!(e.get().is_sensitive());
    /// }
    /// ```
    pub fn get_mut(&mut self) -> &mut T {
        self.inner.get_mut()
    }

    /// Converts the `OccupiedEntry` into a mutable reference to the **first**
    /// value.
    ///
    /// The lifetime of the returned reference is bound to the original map.
    ///
    /// # Panics
    ///
    /// `into_mut` panics if there are no values associated with the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::default();
    /// map.insert("host", "hello.world".parse().unwrap());
    /// map.append("host", "hello.earth".parse().unwrap());
    ///
    /// if let Entry::Occupied(e) = map.entry("host").unwrap() {
    ///     e.into_mut().set_sensitive(true);
    /// }
    ///
    /// assert!(map.get("host").unwrap().is_sensitive());
    /// ```
    pub fn into_mut(self) -> &'a mut T {
        self.inner.into_mut()
    }

    /// Sets the value of the entry.
    ///
    /// All previous values associated with the entry are removed and the first
    /// one is returned. See `insert_mult` for an API that returns all values.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::new();
    /// map.insert("host", "hello.world".parse().unwrap());
    ///
    /// if let Entry::Occupied(mut e) = map.entry("host").unwrap() {
    ///     let mut prev = e.insert("earth".parse().unwrap());
    ///     assert_eq!("hello.world", prev);
    /// }
    ///
    /// assert_eq!("earth", map.get("host").unwrap());
    /// ```
    pub fn insert(&mut self, value: T) -> T {
        self.inner.insert(value)
    }

    /// Sets the value of the entry.
    ///
    /// This function does the same as `insert` except it returns an iterator
    /// that yields all values previously associated with the key.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::new();
    /// map.insert("host", "world".parse().unwrap());
    /// map.append("host", "world2".parse().unwrap());
    ///
    /// if let Entry::Occupied(mut e) = map.entry("host").unwrap() {
    ///     let mut prev = e.insert_mult("earth".parse().unwrap());
    ///     assert_eq!("world", prev.next().unwrap());
    ///     assert_eq!("world2", prev.next().unwrap());
    ///     assert!(prev.next().is_none());
    /// }
    ///
    /// assert_eq!("earth", map.get("host").unwrap());
    /// ```
    pub fn insert_mult(&mut self, value: T) -> ValueDrain<T> {
        ValueDrain { inner: self.inner.insert_mult(value) }
    }

    /// Insert the value into the entry.
    ///
    /// The new value is appended to the end of the entry's value list. All
    /// previous values associated with the entry are retained.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::new();
    /// map.insert("host", "world".parse().unwrap());
    ///
    /// if let Entry::Occupied(mut e) = map.entry("host").unwrap() {
    ///     e.append("earth".parse().unwrap());
    /// }
    ///
    /// let values = map.get_all("host");
    /// let mut i = values.iter();
    /// assert_eq!("world", *i.next().unwrap());
    /// assert_eq!("earth", *i.next().unwrap());
    /// ```
    pub fn append(&mut self, value: T) {
        self.inner.append(value)
    }

    /// Remove the entry from the map.
    ///
    /// All values associated with the entry are removed and the first one is
    /// returned. See `remove_entry_mult` for an API that returns all values.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::new();
    /// map.insert("host", "world".parse().unwrap());
    ///
    /// if let Entry::Occupied(e) = map.entry("host").unwrap() {
    ///     let mut prev = e.remove();
    ///     assert_eq!("world", prev);
    /// }
    ///
    /// assert!(!map.contains_key("host"));
    /// ```
    pub fn remove(self) -> T {
        self.inner.remove()
    }

    /// Remove the entry from the map.
    ///
    /// The key and all values associated with the entry are removed and the
    /// first one is returned. See `remove_entry_mult` for an API that returns
    /// all values.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::new();
    /// map.insert("host", "world".parse().unwrap());
    ///
    /// if let Entry::Occupied(e) = map.entry("host").unwrap() {
    ///     let (key, mut prev) = e.remove_entry();
    ///     assert_eq!("host", key.as_str());
    ///     assert_eq!("world", prev);
    /// }
    ///
    /// assert!(!map.contains_key("host"));
    /// ```
    pub fn remove_entry(self) -> (MetadataKey, T) {
        let (name, value) = self.inner.remove_entry();
        (MetadataKey { inner: name }, value)
    }

    /// Remove the entry from the map.
    ///
    /// The key and all values associated with the entry are removed and
    /// returned.
    pub fn remove_entry_mult(self) -> (MetadataKey, ValueDrain<'a, T>) {
        let (name, value_drain) = self.inner.remove_entry_mult();
        (MetadataKey { inner: name }, ValueDrain { inner: value_drain })
    }

    /// Returns an iterator visiting all values associated with the entry.
    ///
    /// Values are iterated in insertion order.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::new();
    /// map.insert("host", "world".parse().unwrap());
    /// map.append("host", "earth".parse().unwrap());
    ///
    /// if let Entry::Occupied(e) = map.entry("host").unwrap() {
    ///     let mut iter = e.iter();
    ///     assert_eq!(&"world", iter.next().unwrap());
    ///     assert_eq!(&"earth", iter.next().unwrap());
    ///     assert!(iter.next().is_none());
    /// }
    /// ```
    pub fn iter(&self) -> ValueIter<T> {
        ValueIter { inner: self.inner.iter() }
    }

    /// Returns an iterator mutably visiting all values associated with the
    /// entry.
    ///
    /// Values are iterated in insertion order.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::default();
    /// map.insert("host", "world".parse().unwrap());
    /// map.append("host", "earth".parse().unwrap());
    ///
    /// if let Entry::Occupied(mut e) = map.entry("host").unwrap() {
    ///     for e in e.iter_mut() {
    ///         e.set_sensitive(true);
    ///     }
    /// }
    ///
    /// let mut values = map.get_all("host");
    /// let mut i = values.iter();
    /// assert!(i.next().unwrap().is_sensitive());
    /// assert!(i.next().unwrap().is_sensitive());
    /// ```
    pub fn iter_mut(&mut self) -> ValueIterMut<T> {
        ValueIterMut { inner: self.inner.iter_mut() }
    }
}

impl<'a, T> IntoIterator for OccupiedEntry<'a, T> {
    type Item = &'a mut T;
    type IntoIter = ValueIterMut<'a, T>;

    fn into_iter(self) -> ValueIterMut<'a, T> {
        ValueIterMut { inner: self.inner.into_iter() }
    }
}

impl<'a, 'b: 'a, T> IntoIterator for &'b OccupiedEntry<'a, T> {
    type Item = &'a T;
    type IntoIter = ValueIter<'a, T>;

    fn into_iter(self) -> ValueIter<'a, T> {
        self.iter()
    }
}

impl<'a, 'b: 'a, T> IntoIterator for &'b mut OccupiedEntry<'a, T> {
    type Item = &'a mut T;
    type IntoIter = ValueIterMut<'a, T>;

    fn into_iter(self) -> ValueIterMut<'a, T> {
        self.iter_mut()
    }
}

// ===== impl GetAll =====

impl<'a> GetAll<'a> {
    /// Returns an iterator visiting all values associated with the entry.
    ///
    /// Values are iterated in insertion order.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut map = MetadataMap::new();
    /// map.insert("x-host", "hello.world".parse().unwrap());
    /// map.append("x-host", "hello.earth".parse().unwrap());
    ///
    /// let values = map.get_all("x-host");
    /// let mut iter = values.iter();
    /// assert_eq!(&"hello.world", iter.next().unwrap());
    /// assert_eq!(&"hello.earth", iter.next().unwrap());
    /// assert!(iter.next().is_none());
    /// ```
    pub fn iter(&self) -> ValueIter<'a, MetadataValue> {
        ValueIter { inner: self.inner.iter() }
    }
}

impl<'a> PartialEq for GetAll<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.inner.iter().eq(other.inner.iter())
    }
}

impl<'a> IntoIterator for GetAll<'a> {
    type Item = &'a MetadataValue;
    type IntoIter = ValueIter<'a, MetadataValue>;

    fn into_iter(self) -> ValueIter<'a, MetadataValue> {
        ValueIter { inner: self.inner.into_iter() }
    }
}

impl<'a, 'b: 'a> IntoIterator for &'b GetAll<'a> {
    type Item = &'a MetadataValue;
    type IntoIter = ValueIter<'a, MetadataValue>;

    fn into_iter(self) -> ValueIter<'a, MetadataValue> {
        ValueIter {
            inner: (&self.inner).into_iter()
        }
    }
}

// ===== impl IntoMetadataKey / AsMetadataKey =====

mod into_metadata_key {
    use super::{MetadataMap, MetadataValue};
    use metadata_key::MetadataKey;

    /// A marker trait used to identify values that can be used as insert keys
    /// to a `MetadataMap`.
    pub trait IntoMetadataKey: Sealed {}

    // All methods are on this pub(super) trait, instead of `IntoMetadataKey`,
    // so that they aren't publicly exposed to the world.
    //
    // Being on the `IntoMetadataKey` trait would mean users could call
    // `"host".insert(&mut map, "localhost")`.
    //
    // Ultimately, this allows us to adjust the signatures of these methods
    // without breaking any external crate.
    pub trait Sealed {
        #[doc(hidden)]
        fn insert(self, map: &mut MetadataMap, val: MetadataValue) -> Option<MetadataValue>;

        #[doc(hidden)]
        fn append(self, map: &mut MetadataMap, val: MetadataValue) -> bool;
    }

    // ==== impls ====

    impl Sealed for MetadataKey {
        #[doc(hidden)]
        #[inline]
        fn insert(self, map: &mut MetadataMap, val: MetadataValue) -> Option<MetadataValue> {
            map.headers.insert(self.inner, val)
        }

        #[doc(hidden)]
        #[inline]
        fn append(self, map: &mut MetadataMap, val: MetadataValue) -> bool {
            map.headers.append(self.inner, val)
        }
    }

    impl IntoMetadataKey for MetadataKey {}

    impl<'a> Sealed for &'a MetadataKey {
        #[doc(hidden)]
        #[inline]
        fn insert(self, map: &mut MetadataMap, val: MetadataValue) -> Option<MetadataValue> {
            map.headers.insert(&self.inner, val)
        }
        #[doc(hidden)]
        #[inline]
        fn append(self, map: &mut MetadataMap, val: MetadataValue) -> bool {
            map.headers.append(&self.inner, val)
        }
    }

    impl<'a> IntoMetadataKey for &'a MetadataKey {}

    impl Sealed for &'static str {
        #[doc(hidden)]
        #[inline]
        fn insert(self, map: &mut MetadataMap, val: MetadataValue) -> Option<MetadataValue> {
            map.headers.insert(self, val)
        }
        #[doc(hidden)]
        #[inline]
        fn append(self, map: &mut MetadataMap, val: MetadataValue) -> bool {
            map.headers.append(self, val)
        }
    }

    impl IntoMetadataKey for &'static str {}
}

mod as_metadata_key {
    use super::{MetadataMap, MetadataValue};
    use metadata_key::MetadataKey;
    use http::header::{Entry, GetAll, InvalidHeaderName};

    /// A marker trait used to identify values that can be used as search keys
    /// to a `MetadataMap`.
    pub trait AsMetadataKey: Sealed {}

    // All methods are on this pub(super) trait, instead of `AsMetadataKey`,
    // so that they aren't publicly exposed to the world.
    //
    // Being on the `AsMetadataKey` trait would mean users could call
    // `"host".find(&map)`.
    //
    // Ultimately, this allows us to adjust the signatures of these methods
    // without breaking any external crate.
    pub trait Sealed {
        #[doc(hidden)]
        fn get(self, map: &MetadataMap) -> Option<&MetadataValue>;

        #[doc(hidden)]
        fn get_mut(self, map: &mut MetadataMap) -> Option<&mut MetadataValue>;

        #[doc(hidden)]
        fn get_all(self, map: &MetadataMap) -> GetAll<MetadataValue>;

        #[doc(hidden)]
        fn contains_key(&self, map: &MetadataMap) -> bool;

        #[doc(hidden)]
        fn entry(self, map: &mut MetadataMap) -> Result<Entry<MetadataValue>, InvalidHeaderName>;

        #[doc(hidden)]
        fn remove(self, map: &mut MetadataMap) -> Option<MetadataValue>;
    }

    // ==== impls ====

    impl Sealed for MetadataKey {
        #[doc(hidden)]
        #[inline]
        fn get(self, map: &MetadataMap) -> Option<&MetadataValue> {
            map.headers.get(self.inner)
        }

        #[doc(hidden)]
        #[inline]
        fn get_mut(self, map: &mut MetadataMap) -> Option<&mut MetadataValue> {
            map.headers.get_mut(self.inner)
        }

        #[doc(hidden)]
        #[inline]
        fn get_all(self, map: &MetadataMap) -> GetAll<MetadataValue> {
            map.headers.get_all(self.inner)
        }

        #[doc(hidden)]
        #[inline]
        fn contains_key(&self, map: &MetadataMap) -> bool {
            map.headers.contains_key(&self.inner)
        }

        #[doc(hidden)]
        #[inline]
        fn entry(self, map: &mut MetadataMap) -> Result<Entry<MetadataValue>, InvalidHeaderName> {
            map.headers.entry(self.inner)
        }

        #[doc(hidden)]
        #[inline]
        fn remove(self, map: &mut MetadataMap) -> Option<MetadataValue> {
            map.headers.remove(self.inner)
        }
    }

    impl AsMetadataKey for MetadataKey {}

    impl<'a> Sealed for &'a MetadataKey {
        #[doc(hidden)]
        #[inline]
        fn get(self, map: &MetadataMap) -> Option<&MetadataValue> {
            map.headers.get(&self.inner)
        }

        #[doc(hidden)]
        #[inline]
        fn get_mut(self, map: &mut MetadataMap) -> Option<&mut MetadataValue> {
            map.headers.get_mut(&self.inner)
        }

        #[doc(hidden)]
        #[inline]
        fn get_all(self, map: &MetadataMap) -> GetAll<MetadataValue> {
            map.headers.get_all(&self.inner)
        }

        #[doc(hidden)]
        #[inline]
        fn contains_key(&self, map: &MetadataMap) -> bool {
            map.headers.contains_key(&self.inner)
        }

        #[doc(hidden)]
        #[inline]
        fn entry(self, map: &mut MetadataMap) -> Result<Entry<MetadataValue>, InvalidHeaderName> {
            map.headers.entry(&self.inner)
        }

        #[doc(hidden)]
        #[inline]
        fn remove(self, map: &mut MetadataMap) -> Option<MetadataValue> {
            map.headers.remove(&self.inner)
        }
    }

    impl<'a> AsMetadataKey for &'a MetadataKey {}

    impl<'a> Sealed for &'a str {
        #[doc(hidden)]
        #[inline]
        fn get(self, map: &MetadataMap) -> Option<&MetadataValue> {
            map.headers.get(self)
        }

        #[doc(hidden)]
        #[inline]
        fn get_mut(self, map: &mut MetadataMap) -> Option<&mut MetadataValue> {
            map.headers.get_mut(self)
        }

        #[doc(hidden)]
        #[inline]
        fn get_all(self, map: &MetadataMap) -> GetAll<MetadataValue> {
            map.headers.get_all(self)
        }

        #[doc(hidden)]
        #[inline]
        fn contains_key(&self, map: &MetadataMap) -> bool {
            map.headers.contains_key(*self)
        }

        #[doc(hidden)]
        #[inline]
        fn entry(self, map: &mut MetadataMap) -> Result<Entry<MetadataValue>, InvalidHeaderName> {
            map.headers.entry(self)
        }

        #[doc(hidden)]
        #[inline]
        fn remove(self, map: &mut MetadataMap) -> Option<MetadataValue> {
            map.headers.remove(self)
        }
    }

    impl<'a> AsMetadataKey for &'a str {}

    impl Sealed for String {
        #[doc(hidden)]
        #[inline]
        fn get(self, map: &MetadataMap) -> Option<&MetadataValue> {
            map.headers.get(self.as_str())
        }

        #[doc(hidden)]
        #[inline]
        fn get_mut(self, map: &mut MetadataMap) -> Option<&mut MetadataValue> {
            map.headers.get_mut(self.as_str())
        }

        #[doc(hidden)]
        #[inline]
        fn get_all(self, map: &MetadataMap) -> GetAll<MetadataValue> {
            map.headers.get_all(self.as_str())
        }

        #[doc(hidden)]
        #[inline]
        fn contains_key(&self, map: &MetadataMap) -> bool {
            map.headers.contains_key(self.as_str())
        }

        #[doc(hidden)]
        #[inline]
        fn entry(self, map: &mut MetadataMap) -> Result<Entry<MetadataValue>, InvalidHeaderName> {
            map.headers.entry(self.as_str())
        }

        #[doc(hidden)]
        #[inline]
        fn remove(self, map: &mut MetadataMap) -> Option<MetadataValue> {
            map.headers.remove(self.as_str())
        }
    }

    impl AsMetadataKey for String {}

    impl<'a> Sealed for &'a String {
        #[doc(hidden)]
        #[inline]
        fn get(self, map: &MetadataMap) -> Option<&MetadataValue> {
            map.headers.get(self.as_str())
        }

        #[doc(hidden)]
        #[inline]
        fn get_mut(self, map: &mut MetadataMap) -> Option<&mut MetadataValue> {
            map.headers.get_mut(self.as_str())
        }

        #[doc(hidden)]
        #[inline]
        fn get_all(self, map: &MetadataMap) -> GetAll<MetadataValue> {
            map.headers.get_all(self.as_str())
        }

        #[doc(hidden)]
        #[inline]
        fn contains_key(&self, map: &MetadataMap) -> bool {
            map.headers.contains_key(self.as_str())
        }

        #[doc(hidden)]
        #[inline]
        fn entry(self, map: &mut MetadataMap) -> Result<Entry<MetadataValue>, InvalidHeaderName> {
            map.headers.entry(self.as_str())
        }

        #[doc(hidden)]
        #[inline]
        fn remove(self, map: &mut MetadataMap) -> Option<MetadataValue> {
            map.headers.remove(self.as_str())
        }
    }

    impl<'a> AsMetadataKey for &'a String {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_headers_takes_http_headers() {
        let mut http_map = http::HeaderMap::new();
        http_map.insert("x-host", "example.com".parse().unwrap());

        let map = MetadataMap::from_headers(http_map);

        assert_eq!(map.get("x-host").unwrap(), "example.com");
    }
}
