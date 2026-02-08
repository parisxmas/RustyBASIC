' TCP server example
WIFI.CONNECT "MySSID", "MyPassword"
DELAY 3000
TCP.LISTEN 8080
PRINT "Listening on port 8080..."
TCP.ACCEPT client%
PRINT "Client connected: "; client%
TCP.RECEIVE$ request$
PRINT "Got: "; request$
TCP.SEND "HTTP/1.0 200 OK\r\n\r\nHello from RustyBASIC!\r\n"
TCP.CLOSE
