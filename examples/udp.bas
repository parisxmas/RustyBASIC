' UDP Networking Example
' Send and receive UDP datagrams

WIFI.CONNECT "MyNetwork", "password123"

UDP.INIT 8888
UDP.SEND "192.168.1.100", 9999, "Hello UDP!"

DIM msg$
UDP.RECEIVE msg$
PRINT "Received: "; msg$
