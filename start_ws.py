#!/usr/bin/python
import socketserver
import socket
import http.server
import sys

class CORSRequestHandler(http.server.SimpleHTTPRequestHandler):
    def send_my_headers(self):
        print("This is working :/")
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header('Cache-Control', 'no-store, must-revalidate')
        self.send_header('Expires', '0')
        http.server.SimpleHTTPRequestHandler.end_headers(self)

    def end_headers(self):
        self.send_my_headers()

print('Server listening on port 80...')
httpd = socketserver.TCPServer(('', 80), CORSRequestHandler)
httpd.serve_forever()
