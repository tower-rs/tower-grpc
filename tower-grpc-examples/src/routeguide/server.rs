#![allow(dead_code)]
#![allow(unused_variables)]

extern crate bytes;
extern crate env_logger;
extern crate futures;
#[macro_use]
extern crate log;
extern crate prost;
#[macro_use]
extern crate prost_derive;
extern crate tokio;
extern crate tower_h2;
extern crate tower_grpc;

extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

mod data;
pub mod routeguide {
    include!(concat!(env!("OUT_DIR"), "/routeguide.rs"));
}
use routeguide::{server, Point, Rectangle, Feature, RouteSummary, RouteNote};

use futures::{future, stream, Future, Stream, Sink};
use futures::sync::mpsc;
use tokio::executor::DefaultExecutor;
use tokio::net::TcpListener;
use tower_h2::Server;
use tower_grpc::{Request, Response, Streaming};

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Debug, Clone)]
struct RouteGuide {
    state: Arc<State>,
}

#[derive(Debug)]
struct State {
    features: Vec<routeguide::Feature>,
    notes: Mutex<HashMap<Point, Vec<RouteNote>>>,
}

// Implement hash for Point
impl Hash for Point {
    fn hash<H>(&self, state: &mut H)
    where H: Hasher,
    {
        self.latitude.hash(state);
        self.longitude.hash(state);
    }
}

impl Eq for Point {}

impl routeguide::server::RouteGuide for RouteGuide {
    type GetFeatureFuture = future::FutureResult<Response<Feature>, tower_grpc::Error>;

    /// returns the feature at the given point.
    fn get_feature(&mut self, request: Request<Point>) -> Self::GetFeatureFuture {
        println!("GetFeature = {:?}", request);

        for feature in &self.state.features[..] {
            if feature.location.as_ref() == Some(request.get_ref()) {
                return future::ok(Response::new(feature.clone()));
            }
        }

        // Otherwise, return some other feature?
        let response = Response::new(Feature {
            name: "".to_string(),
            location: None,
        });

        future::ok(response)
    }

    type ListFeaturesStream = Box<Stream<Item = Feature, Error = tower_grpc::Error> + Send>;
    type ListFeaturesFuture = future::FutureResult<Response<Self::ListFeaturesStream>, tower_grpc::Error>;

    /// Lists all features contained within the given bounding Rectangle.
    fn list_features(&mut self, request: Request<Rectangle>) -> Self::ListFeaturesFuture {
        use std::thread;

        println!("ListFeatures = {:?}", request);

        let (tx, rx) = mpsc::channel(4);

        let state = self.state.clone();

        thread::spawn(move || {
            let mut tx = tx.wait();

            for feature in &state.features[..] {
                if in_range(feature.location.as_ref().unwrap(), request.get_ref()) {
                    println!("  => send {:?}", feature);
                    tx.send(feature.clone()).unwrap();
                }
            }

            println!(" /// done sending");
        });

        let rx = rx.map_err(|_| unimplemented!());
        future::ok(Response::new(Box::new(rx)))
    }

    type RecordRouteFuture = Box<Future<Item = Response<RouteSummary>, Error = tower_grpc::Error> + Send>;

    /// Records a route composited of a sequence of points.
    ///
    /// It gets a stream of points, and responds with statistics about the
    /// "trip": number of points,  number of known features visited, total
    /// distance traveled, and total time spent.
    fn record_route(&mut self, request: Request<Streaming<Point>>) -> Self::RecordRouteFuture {
        println!("RecordRoute = {:?}", request);

        let now = Instant::now();
        let state = self.state.clone();

        let response = request.into_inner()
            .map_err(|e| {
                println!("  !!! err={:?}", e);
                e
            })
            // Iterate over all points, building up the route summary
            .fold((RouteSummary::default(), None), move |(mut summary, last_point), point| {
                println!("  ==> Point = {:?}", point);

                // Increment the point count
                summary.point_count += 1;

                // Find features
                for feature in &state.features[..] {
                    if feature.location.as_ref() == Some(&point) {
                        summary.feature_count += 1;
                    }
                }

                // Calculate the distance
                if let Some(ref last_point) = last_point {
                    summary.distance += calc_distance(last_point, &point);
                }

                Ok::<_, tower_grpc::Error>((summary, Some(point)))
            })
            // Map the route summary to a gRPC response
            .map(move |(mut summary, _)| {
                println!("  => Done = {:?}", summary);

                summary.elapsed_time = now.elapsed().as_secs() as i32;
                Response::new(summary)
            })
            ;

        Box::new(response)
    }

    type RouteChatStream = Box<Stream<Item = RouteNote, Error = tower_grpc::Error> + Send>;
    type RouteChatFuture = future::FutureResult<Response<Self::RouteChatStream>, tower_grpc::Error>;

    // Receives a stream of message/location pairs, and responds with a stream
    // of all previous messages at each of those locations.
    fn route_chat(&mut self, request: Request<Streaming<RouteNote>>) -> Self::RouteChatFuture {
        println!("RouteChat = {:?}", request);

        let state = self.state.clone();

        let response = request.into_inner()
            .map(move |note| {
                let location = note.location.clone().unwrap();
                let mut notes = state.notes.lock().unwrap();
                let notes = notes.entry(location)
                    .or_insert(vec![]);

                notes.push(note);

                stream::iter_ok(notes.clone())
            })
            .flatten()
            ;

        future::ok(Response::new(Box::new(response)))
    }
}

fn in_range(point: &Point, rect: &Rectangle) -> bool {
    use std::cmp;

    let lo = rect.lo.as_ref().unwrap();
    let hi = rect.hi.as_ref().unwrap();

    let left = cmp::min(lo.longitude, hi.longitude);
    let right = cmp::max(lo.longitude, hi.longitude);
    let top = cmp::max(lo.latitude, hi.latitude);
    let bottom = cmp::min(lo.latitude, hi.latitude);

    point.longitude >= left &&
        point.longitude <= right &&
        point.latitude >= bottom &&
        point.latitude <= top
}

/// Calculates the distance between two points using the "haversine" formula.
/// This code was taken from http://www.movable-type.co.uk/scripts/latlong.html.
fn calc_distance(p1: &Point, p2: &Point) -> i32 {
    const CORD_FACTOR: f64 = 1e7;
    const R: f64 = 6371000.0; // meters

    let lat1 = p1.latitude as f64 / CORD_FACTOR;
    let lat2 = p2.latitude as f64 / CORD_FACTOR;
    let lng1 = p1.longitude as f64 / CORD_FACTOR;
    let lng2 = p2.longitude as f64 / CORD_FACTOR;

    let lat_rad1 = lat1.to_radians();
    let lat_rad2 = lat2.to_radians();

    let delta_lat = (lat2 - lat1).to_radians();
    let delta_lng = (lng2 - lng1).to_radians();

    let a = (delta_lat / 2f64).sin() * (delta_lat / 2f64).sin() +
        (lat_rad1).cos() * (lat_rad2).cos() *
        (delta_lng / 2f64).sin() * (delta_lng / 2f64).sin();

    let c = 2f64 * a.sqrt().atan2((1f64 - a).sqrt());

    (R * c) as i32
}

pub fn main() {
    let _ = ::env_logger::init();


    let handler = RouteGuide {
        state: Arc::new(State {
            // Load data file
            features: data::load(),
            notes: Mutex::new(HashMap::new()),
        }),
    };

    let new_service = server::RouteGuideServer::new(handler);

    let h2_settings = Default::default();
    let mut h2 = Server::new(new_service, h2_settings, DefaultExecutor::current());

    let addr = "127.0.0.1:10000".parse().unwrap();
    let bind = TcpListener::bind(&addr).expect("bind");

    println!("listining on {:?}", addr);

    let serve = bind.incoming()
        .for_each(move |sock| {
            if let Err(e) = sock.set_nodelay(true) {
                return Err(e);
            }

            let serve = h2.serve(sock);
            tokio::spawn(serve.map_err(|e| error!("h2 error: {:?}", e)));

            Ok(())
        })
        .map_err(|e| eprintln!("accept error: {}", e));

    tokio::run(serve);
}
