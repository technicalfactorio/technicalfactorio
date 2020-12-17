# Whitelist Signal Filter

The left input is the "filter" input, while the right is the "value" input. The
output is all the values on the value input for which the filter is non-zero.

This is a fully pipelined design with two ticks latency. It supports all 2^32
possible values on either input.

