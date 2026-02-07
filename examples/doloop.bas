' DO...LOOP examples (QBASIC style)

DIM count AS INTEGER
DIM sum AS INTEGER

' DO WHILE...LOOP (pre-condition)
count = 1
sum = 0
DO WHILE count <= 10
    sum = sum + count
    count = count + 1
LOOP
PRINT "Sum 1..10 ="; sum

' DO...LOOP UNTIL (post-condition)
count = 10
DO
    PRINT count;
    count = count - 1
LOOP UNTIL count < 1
PRINT

' DO...LOOP with EXIT DO
count = 0
DO
    count = count + 1
    IF count = 5 THEN EXIT DO
LOOP
PRINT "Exited at count ="; count

END
