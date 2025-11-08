build:
	docker build -t cm3588-build .
	mkdir -p dist
	docker run --rm --name cm3588-build -v ./dist:/build cm3588-build
	cp cm3588-fan.service ./dist

release:
	git-cliff -l | wl-copy
