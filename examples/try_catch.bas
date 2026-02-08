' TRY/CATCH example
PRINT "Before try"

TRY
    PRINT "Inside try block"
    PRINT "This should work fine"
CATCH err
    PRINT "Caught error: "; err
END TRY

PRINT "After try/catch"
END
