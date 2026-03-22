CARGO = ~/.cargo/bin/cargo

.PHONY: release-patch release-minor release-major

release-patch:
	$(CARGO) release patch --execute
	git push && git push --tags

release-minor:
	$(CARGO) release minor --execute
	git push && git push --tags

release-major:
	$(CARGO) release major --execute
	git push && git push --tags
