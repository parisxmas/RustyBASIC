' DATA/READ/RESTORE Example
' Look up planet names and distances from the Sun

DATA "Mercury", 57.9
DATA "Venus", 108.2
DATA "Earth", 149.6
DATA "Mars", 227.9

FOR i = 1 TO 4
    READ planet$, distance
    PRINT planet$; " is "; distance; " million km from the Sun"
NEXT i

RESTORE
READ first$, d
PRINT "First planet again: "; first$
