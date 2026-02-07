' FizzBuzz in QBASIC style

DIM i AS INTEGER

FOR i = 1 TO 100
    SELECT CASE 0
        CASE i MOD 15
            PRINT "FizzBuzz"
        CASE i MOD 3
            PRINT "Fizz"
        CASE i MOD 5
            PRINT "Buzz"
        CASE ELSE
            PRINT i
    END SELECT
NEXT i

END
