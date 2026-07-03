if not exist run-logs mkdir run-logs
cargo check --message-format=short > run-logs/cargo_check_out.txt 2>&1
echo done >> run-logs/cargo_check_out.txt