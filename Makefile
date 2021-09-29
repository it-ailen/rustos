DOCKER_NAME ?= allenzou/rustos
.PHONY: docker build_docker

docker:
	docker run --rm -it --mount type=bind,source=$(shell pwd),destination=/mnt -w /mnt/os --cap-add=SYS_PTRACE --security-opt seccomp=unconfined ${DOCKER_NAME}

build_docker: 
	docker build -t ${DOCKER_NAME} .
