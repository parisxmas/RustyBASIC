' ESP-NOW Peer-to-Peer Messaging
' Send and receive messages between two ESP32 boards

ESPNOW.INIT

' Send a message to a peer
LET peer$ = "AA:BB:CC:DD:EE:FF"
LET msg$ = "Hello from RustyBASIC!"
ESPNOW.SEND peer$, msg$
PRINT "Sent: "; msg$

' Wait for a reply
ESPNOW.RECEIVE reply$
PRINT "Received: "; reply$
