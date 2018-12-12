#!/usr/bin/env bash

# check we're in the grin root
if [ ! -f "LICENSE" ] ; then
	echo "Script must be run from Grin-miner's root directory"
	exit 1
fi

echo "Going to package and publish each crate, if you're not logged in crates.io (missing ~/.cargo/credentials, this will fail."

read -p "Continue? " -n 1 -r
if [[ ! $REPLY =~ ^[Yy]$ ]]
then
	printf "\nbye\n"
	exit 1
fi

echo
crates=( config cuckoo-miner plugin util )

for crate in "${crates[@]}"
do
	echo "** Publishing $crate"
	cd $crate
	cargo package
	cargo publish
	cd ..
done

cargo package
cargo publish

echo "Done."
