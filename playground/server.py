"""GAME Playground server — static files + API proxy for AI generation.

Usage: python playground/server.py
Serves on http://localhost:8787
"""

import json
import sys
from http.server import HTTPServer, SimpleHTTPRequestHandler
from urllib.request import Request, urlopen
from urllib.error import HTTPError, URLError
import os

PORT = 8787

class PlaygroundHandler(SimpleHTTPRequestHandler):
    """Serves static files from repo root AND proxies AI API requests."""

    def __init__(self, *args, **kwargs):
        # Serve from repo root (parent of playground/)
        super().__init__(*args, directory=os.path.dirname(os.path.dirname(os.path.abspath(__file__))), **kwargs)

    def do_OPTIONS(self):
        """Handle CORS preflight for proxy endpoints."""
        if self.path.startswith('/api/'):
            self.send_response(204)
            self.send_header('Access-Control-Allow-Origin', '*')
            self.send_header('Access-Control-Allow-Methods', 'POST, OPTIONS')
            self.send_header('Access-Control-Allow-Headers', 'Content-Type')
            self.send_header('Access-Control-Max-Age', '86400')
            self.end_headers()
        else:
            super().do_OPTIONS()

    def do_POST(self):
        """Proxy AI API requests to avoid browser CORS issues."""
        if self.path == '/api/anthropic':
            self._proxy_anthropic()
        elif self.path == '/api/openai':
            self._proxy_openai()
        else:
            self.send_error(404)

    def _proxy_anthropic(self):
        try:
            content_length = int(self.headers.get('Content-Length', 0))
            body = self.rfile.read(content_length)
            data = json.loads(body)

            api_key = data.get('apiKey', '')
            payload = {
                'model': data.get('model', 'claude-sonnet-4-6'),
                'max_tokens': data.get('max_tokens', 4096),
                'system': data.get('system', ''),
                'messages': data.get('messages', []),
            }

            req = Request(
                'https://api.anthropic.com/v1/messages',
                data=json.dumps(payload).encode('utf-8'),
                headers={
                    'Content-Type': 'application/json',
                    'x-api-key': api_key,
                    'anthropic-version': '2023-06-01',
                },
                method='POST',
            )

            resp = urlopen(req)
            result = resp.read()

            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Access-Control-Allow-Origin', '*')
            self.end_headers()
            self.wfile.write(result)

        except HTTPError as e:
            error_body = e.read()
            self.send_response(e.code)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Access-Control-Allow-Origin', '*')
            self.end_headers()
            self.wfile.write(error_body)
        except URLError as e:
            self.send_response(502)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Access-Control-Allow-Origin', '*')
            self.end_headers()
            self.wfile.write(json.dumps({'error': str(e.reason)}).encode())
        except Exception as e:
            self.send_response(500)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Access-Control-Allow-Origin', '*')
            self.end_headers()
            self.wfile.write(json.dumps({'error': str(e)}).encode())

    def _proxy_openai(self):
        try:
            content_length = int(self.headers.get('Content-Length', 0))
            body = self.rfile.read(content_length)
            data = json.loads(body)

            api_key = data.get('apiKey', '')
            payload = {
                'model': data.get('model', 'gpt-4o'),
                'max_tokens': data.get('max_tokens', 4096),
                'messages': data.get('messages', []),
            }

            req = Request(
                'https://api.openai.com/v1/chat/completions',
                data=json.dumps(payload).encode('utf-8'),
                headers={
                    'Content-Type': 'application/json',
                    'Authorization': f'Bearer {api_key}',
                },
                method='POST',
            )

            resp = urlopen(req)
            result = resp.read()

            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Access-Control-Allow-Origin', '*')
            self.end_headers()
            self.wfile.write(result)

        except HTTPError as e:
            error_body = e.read()
            self.send_response(e.code)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Access-Control-Allow-Origin', '*')
            self.end_headers()
            self.wfile.write(error_body)
        except Exception as e:
            self.send_response(500)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Access-Control-Allow-Origin', '*')
            self.end_headers()
            self.wfile.write(json.dumps({'error': str(e)}).encode())

    def log_message(self, format, *args):
        # Quieter logging — skip static file GETs, show API proxying
        if self.path.startswith('/api/'):
            sys.stderr.write(f"[proxy] {self.path} {args[0] if args else ''}\n")


if __name__ == '__main__':
    print(f"GAME Playground -> http://localhost:{PORT}/playground/")
    print(f"API proxy ready  -> /api/anthropic, /api/openai")
    httpd = HTTPServer(('', PORT), PlaygroundHandler)
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        print("\nShutting down.")
        httpd.shutdown()
