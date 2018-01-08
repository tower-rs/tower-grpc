FROM golang:1.9 as build

ENV PACKAGE google.golang.org/grpc/interop/server
ENV PACKAGEDIR $GOPATH/src/$PACKAGE

RUN go get -u $PACKAGE

WORKDIR $PACKAGEDIR
RUN CGO_ENABLED=0 GOOS=linux go build -a -installsuffix cgo -o server .

FROM alpine:latest

ENV PACKAGE google.golang.org/grpc/interop/server
ENV PACKAGEDIR $GOPATH/src/$PACKAGE

WORKDIR /root/
COPY --from=build /go/src/google.golang.org/grpc/interop/server/server .

ENTRYPOINT ["./server"]