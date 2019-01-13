plugins_dir=$(egrep '^miner_plugin_dir' grin-miner.toml | awk '{ print $NF }' | xargs echo)
if [ -z "$plugins_dir" ]; then
	plugins_dir="target/debug/plugins"
fi

# Install ocl_cuckatoo
cd ocl_cuckatoo
cargo build --release
cd ..
cp target/release/libocl_cuckatoo.so $plugins_dir/ocl_cuckatoo.cuckooplugin

# Install ocl_cuckaroo
cd ocl_cuckaroo
cargo build --release
cd ..
cp target/release/libocl_cuckaroo.so $plugins_dir/ocl_cuckaroo.cuckooplugin
