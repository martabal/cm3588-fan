.PHONY: build

build:
	docker build -t cm3588-build .
	mkdir -p build
	docker run --rm --name cm3588-build -v ./build:/build cm3588-build
	cp cm3588-fan.service ./build