' Servo Motor Example
' Control a servo on GPIO pin 5

SERVO.ATTACH 0, 5

' Sweep from 0 to 180 degrees
FOR angle = 0 TO 180 STEP 10
    SERVO.WRITE 0, angle
    DELAY 100
NEXT angle

' Return to center
SERVO.WRITE 0, 90
