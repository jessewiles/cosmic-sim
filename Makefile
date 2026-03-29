CARGO = ~/.cargo/bin/cargo

.PHONY: release-patch release-minor release-major

release-patch:
	$(CARGO) release patch --execute --no-confirm && git push && git push --tags

release-minor:
	$(CARGO) release minor --execute --no-confirm && git push && git push --tags

release-major:
	$(CARGO) release major --execute --no-confirm && git push && git push --tags
