#!/usr/bin/env python3
"""
OAuth 2.0 loopback listener for md-to-gdoc.

Binds to 127.0.0.1 on an OS-assigned port, prints PORT=<port> to stderr,
waits for one GET request to / with ?code=... query, returns "you can close
this tab" to the browser, and prints CODE=<code> to stdout (or ERROR=<error>).
Then exits.

Usage (from bash):
    python3 _auth_listener.py 2> >(while read l; do
        case "$l" in PORT=*) port="${l#PORT=}";; esac
    done)
"""
import http.server
import socket
import sys
import urllib.parse


class _OneShotHandler(http.server.BaseHTTPRequestHandler):
    captured = None  # ("code", value) or ("error", value)

    def do_GET(self):  # noqa: N802 (http.server API)
        parsed = urllib.parse.urlparse(self.path)
        params = urllib.parse.parse_qs(parsed.query)
        if "code" in params:
            _OneShotHandler.captured = ("code", params["code"][0])
            body = b"<h1>md-to-gdoc: authorized</h1><p>You can close this tab.</p>"
        elif "error" in params:
            _OneShotHandler.captured = ("error", params["error"][0])
            body = (
                b"<h1>md-to-gdoc: authorization failed</h1>"
                b"<p>" + params["error"][0].encode() + b"</p>"
            )
        else:
            _OneShotHandler.captured = ("error", "no_code_in_callback")
            body = b"<h1>md-to-gdoc: no code in callback</h1>"
        self.send_response(200)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *_args, **_kwargs):  # silence default access log
        return


def main() -> int:
    probe = socket.socket()
    probe.bind(("127.0.0.1", 0))
    port = probe.getsockname()[1]
    probe.close()
    print(f"PORT={port}", file=sys.stderr, flush=True)

    server = http.server.HTTPServer(("127.0.0.1", port), _OneShotHandler)
    server.handle_request()  # one shot
    server.server_close()

    if _OneShotHandler.captured is None:
        print("ERROR=no_request_received")
        return 1
    kind, value = _OneShotHandler.captured
    if kind == "code":
        print(f"CODE={value}")
        return 0
    print(f"ERROR={value}")
    return 1


if __name__ == "__main__":
    sys.exit(main())
