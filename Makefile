build:
	cargo build

release:
	cargo build --release

run:
	cargo run

gen-plist:
	test -n "$(REPO_PATH)"
	test -n "$(SIGN_EMAIL)"
	test -n "$(SIGN_NAME)"
	test -n "$(REPO_REMOTE)"
	test -n "$(SSH_KEY_PATH)"
	test -n "$(INFO_LOG_PATH)"
	test -n "$(ERROR_LOG_PATH)"
	envsubst < backer-rs.plist.tmpl > backer-rs.plist

install-mac: gen-plist
	cp backer-rs.plist /Library/LaunchDaemons/backer-rs.plist
	launchctl load /Library/LaunchDaemons/backer-rs.plist

uninstall-mac:
	launchctl unload /Library/LaunchDaemons/backer-rs.plist

start-svc:
	launchctl start backer-rs

stop-svc:
	launchctl stop backer-rs