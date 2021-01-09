# Pairwise multipliction

Takes a full frame of inputs on both the red and green input wires and
multiplies each of the respective signals together. (`A1*A2`, `B1*B2`, etc)

Outputs the results as a full frame of the respective signals.

The design is fully pipelined and has two ticks latency.

Accepted input values are those for which the following conditions hold:

* `(A + B)^2 < (2^31)-1`
* `A^2 < (2^31)-1`
* `B^2 < (2^31)-1`

For values for which the conditions do not hold, the output will be incorrect.


Inner workings:

```
AB = (A^2 + B^2) / -2 + (A+B)^2 / 2
-2AB = A^2 + B^2 - (A+B)^2
-2AB = A^2 + B^2 - A^2 - 2AB - B^2
-2AB = -2AB
```
