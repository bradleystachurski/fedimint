for i in {1..10}; do
  RUST_BACKTRACE=1 \
  FM_DEVIMINT_DISABLE_MODULE_LNV2=1 \
  just test-upgrades v0.4.0 current &>> results.log
  echo "sleeping to give daemons time to shutdown" &>> results.log
  sleep 10
done


