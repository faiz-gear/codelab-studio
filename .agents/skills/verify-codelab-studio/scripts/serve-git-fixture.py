#!/usr/bin/env python3
"""Serve a disposable Course Pack over loopback Git smart HTTP."""

import os
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path
import subprocess
import sys
from urllib.parse import urlsplit


class GitHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        self.serve_git("/course.git/info/refs")

    def do_POST(self):
        self.serve_git("/course.git/git-upload-pack")

    def serve_git(self, route):
        request = urlsplit(self.path)
        if request.path != route:
            self.send_error(404)
            return

        env = {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}
        env.update(
            GIT_PROJECT_ROOT=str(self.server.repo_root),
            GIT_HTTP_EXPORT_ALL="1",
            PATH_INFO=request.path,
            QUERY_STRING=request.query,
            REQUEST_METHOD=self.command,
            CONTENT_TYPE=self.headers.get("Content-Type", ""),
            CONTENT_LENGTH=self.headers.get("Content-Length", "0"),
            HTTP_GIT_PROTOCOL=self.headers.get("Git-Protocol", ""),
        )
        result = subprocess.run(
            ["git", "http-backend"],
            env=env,
            input=self.rfile.read(int(env["CONTENT_LENGTH"])),
            stdout=subprocess.PIPE,
        )
        headers, separator, body = result.stdout.partition(b"\r\n\r\n")
        if result.returncode or not separator:
            self.send_error(502)
            return

        status = 200
        response_headers = []
        for header in headers.decode("iso-8859-1").split("\r\n"):
            name, value = header.split(":", 1)
            if name.lower() == "status":
                status = int(value.split()[0])
            else:
                response_headers.append((name, value.strip()))

        self.send_response(status)
        for name, value in response_headers:
            self.send_header(name, value)
        self.end_headers()
        self.wfile.write(body)


if __name__ == "__main__":
    if len(sys.argv) != 2 or not (Path(sys.argv[1]) / "course.git").is_dir():
        sys.exit(f"Usage: {sys.argv[0]} DIRECTORY_CONTAINING_COURSE.GIT")
    with HTTPServer(("127.0.0.1", 0), GitHandler) as server:
        server.repo_root = Path(sys.argv[1]).resolve()
        print(f"http://127.0.0.1:{server.server_port}/course.git", flush=True)
        try:
            server.serve_forever()
        except KeyboardInterrupt:
            pass
