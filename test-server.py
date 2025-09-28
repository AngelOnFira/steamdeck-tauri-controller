#!/usr/bin/env python3
"""
Simple test server for Steam Deck Controller Light Show app.
Receives controller input data and prints it to console.
"""

from http.server import HTTPServer, BaseHTTPRequestHandler
import json
from datetime import datetime

class LightShowHandler(BaseHTTPRequestHandler):
    def do_OPTIONS(self):
        self.send_response(200)
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS')
        self.send_header('Access-Control-Allow-Headers', 'Content-Type')
        self.end_headers()

    def do_POST(self):
        if self.path == '/light-control':
            content_length = int(self.headers['Content-Length'])
            post_data = self.rfile.read(content_length)
            
            try:
                data = json.loads(post_data.decode('utf-8'))
                timestamp = datetime.fromtimestamp(data.get('timestamp', 0) / 1000)
                
                print(f"\n[{timestamp.strftime('%H:%M:%S.%f')[:-3]}] Controller Input Received:")
                print(f"  Controller ID: {data.get('controller_id', 'Unknown')}")
                print(f"  Action: {data.get('action', 'Unknown')}")
                
                # Here you would trigger your light show actions
                # For now, just echo back success
                
                self.send_response(200)
                self.send_header('Content-type', 'application/json')
                self.send_header('Access-Control-Allow-Origin', '*')
                self.end_headers()
                
                response = {
                    'status': 'success',
                    'message': f"Received {data.get('action', 'unknown')} command"
                }
                self.wfile.write(json.dumps(response).encode())
                
            except Exception as e:
                print(f"Error processing request: {e}")
                self.send_response(400)
                self.send_header('Content-type', 'application/json')
                self.send_header('Access-Control-Allow-Origin', '*')
                self.end_headers()
                
                response = {'status': 'error', 'message': str(e)}
                self.wfile.write(json.dumps(response).encode())
        else:
            self.send_response(404)
            self.end_headers()
    
    def log_message(self, format, *args):
        # Suppress default logging
        pass

def run_server(port=8080):
    server_address = ('', port)
    httpd = HTTPServer(server_address, LightShowHandler)
    
    print(f"Light Show Test Server")
    print(f"======================")
    print(f"Listening on port {port}")
    print(f"Endpoint: http://localhost:{port}/light-control")
    print(f"\nPress Ctrl+C to stop the server")
    print(f"\nWaiting for controller inputs...")
    
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        print("\n\nShutting down server...")
        httpd.shutdown()

if __name__ == '__main__':
    run_server()