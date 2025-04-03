.PHONY: build

build:
	docker build -t cm3588-build .
	docker run --rm --name cm3588-build -v ./build:/build cm3588-build
	cp fan-cm3588.service ./build