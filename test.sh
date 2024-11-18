#!/bin/bash
function exec_verbose() {
    echo "> exec $@"
    $@  
}

echo "Building..."
exec_verbose cargo build
exec_verbose mkdir -p ./target/test/
echo "Done Building, preparing for tests"
exec_verbose sudo cp ./target/debug/nyado ./target/test/
exec_verbose sudo chown root:root ./target/test/nyado
exec_verbose sudo chmod u+s ./target/test/nyado 
exec_verbose ./target/test/nyado $@
