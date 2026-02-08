' NeoPixel (WS2812) Rainbow Cycle
' Connects an 8-LED strip on GPIO pin 8

CONST NUM_LEDS = 8
CONST PIN = 8

LED.SETUP PIN, NUM_LEDS

' Rainbow cycle
FOR cycle = 0 TO 2
    FOR i = 0 TO NUM_LEDS - 1
        LET r = (i * 32 + cycle * 85) MOD 256
        LET g = (i * 64 + cycle * 85) MOD 256
        LET b = (255 - i * 32 + cycle * 85) MOD 256
        LED.SET i, r, g, b
    NEXT i
    LED.SHOW
    DELAY 500
NEXT cycle

LED.CLEAR
PRINT "Rainbow complete!"
