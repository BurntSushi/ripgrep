#!/bin/bash -ex

# Number of seconds to run the script. Default to 5 minutes.
duration="${1:-300}"
start_time=$SECONDS
elapsed_time=0

while (( $elapsed_time < $duration )); do
    for target in $(cargo fuzz_list); do cargo fuzz_run $target --release -- -max_total_time=60 -verbosity=0; done
    
    elapsed_time=$(($SECONDS - $start_time))
done
