.PHONY: pull up deploy

pull:
	docker compose -f compose.prod.yaml pull

up:
	docker compose -f compose.prod.yaml up -d --force-recreate --remove-orphans

deploy: pull up
	echo "Deploying to production..."