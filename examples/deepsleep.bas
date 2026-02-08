' Deep Sleep Example
' Read a sensor value, print it, then sleep for 10 seconds

PRINT "Waking up..."

ADC.READ 0, sensorValue
PRINT "Sensor value: "; sensorValue

PRINT "Going to deep sleep for 10 seconds..."
DEEPSLEEP 10000
