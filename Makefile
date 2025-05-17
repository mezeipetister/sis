.PHONY: deploy

export NODE_OPTIONS=--openssl-legacy-provider

.PHONY: up all $(SUBDIRS) deploy

deploy:
	