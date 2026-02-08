' WebSocket client example
WIFI.CONNECT "MySSID", "MyPassword"
DELAY 3000
WS.CONNECT "ws://echo.websocket.org"
WS.SEND "Hello WebSocket!"
DELAY 1000
WS.RECEIVE$ msg$
PRINT "Received: "; msg$
WS.CLOSE
