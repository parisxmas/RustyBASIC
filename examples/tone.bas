' Tone / Buzzer Example
' Play a simple melody on GPIO pin 8

' Note frequencies (Hz)
DATA 262, 294, 330, 349, 392, 440, 494, 523

FOR i = 1 TO 8
    READ freq%
    TONE 8, freq%, 300
    DELAY 350
NEXT i
