' Web server example
WIFI.CONNECT "MySSID", "MyPassword"
DELAY 3000
WEB.START 80
PRINT "Web server started on port 80"
WEB.WAIT$ path$
PRINT "Request for: "; path$
WEB.BODY$ body$
PRINT "Body: "; body$
WEB.REPLY 200, "Hello from RustyBASIC!"
WEB.STOP
PRINT "Server stopped"
