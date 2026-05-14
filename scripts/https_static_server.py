#!/usr/bin/env python3
import argparse
import http.server
import ssl
from functools import partial


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--bind", default="0.0.0.0")
    parser.add_argument("--port", type=int, default=3443)
    parser.add_argument("--root", required=True)
    parser.add_argument("--cert", required=True)
    parser.add_argument("--key", required=True)
    args = parser.parse_args()

    handler = partial(http.server.SimpleHTTPRequestHandler, directory=args.root)
    httpd = http.server.ThreadingHTTPServer((args.bind, args.port), handler)
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    context.load_cert_chain(certfile=args.cert, keyfile=args.key)
    httpd.socket = context.wrap_socket(httpd.socket, server_side=True)
    print(f"https static server listening on https://{args.bind}:{args.port}/ root={args.root}", flush=True)
    httpd.serve_forever()


if __name__ == "__main__":
    main()
