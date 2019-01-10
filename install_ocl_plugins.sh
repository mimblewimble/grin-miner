plugins_dir=`grep miner_plugin_dir grin-miner.toml |  cut -d "\"" -f2 | cut -d "\"" -f1`
if [ -z "$plugins_dir" ];
	then plugins_dir="target/debug/plugins"
fi
cd ocl_cuckatoo
cargo build --release
cd ..
cp target/release/libocl_cuckatoo.so $plugins_dir/ocl_cuckatoo.cuckooplugin
cd ocl_cuckaroo
cargo build --release
cd ..
cp target/release/libocl_cuckaroo.so $plugins_dir/ocl_cuckaroo.cuckooplugin

