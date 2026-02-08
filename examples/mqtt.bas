' MQTT pub/sub example for ESP32-C3
DIM status AS INTEGER
DIM msg$ AS STRING

WIFI.CONNECT "MyNetwork", "MyPassword"
DELAY 3000
WIFI.STATUS status

IF status = 1 THEN
    MQTT.CONNECT "mqtt://broker.hivemq.com", 1883
    MQTT.SUBSCRIBE "rustybasic/test"
    MQTT.PUBLISH "rustybasic/test", "Hello from RustyBASIC!"
    DELAY 2000
    MQTT.RECEIVE msg$
    PRINT "Received: "; msg$
    MQTT.DISCONNECT
END IF

END
