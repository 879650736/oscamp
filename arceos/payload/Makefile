SUB_DIRS=origin hello_c fileops_c mapfile_c skernel skernel2
ARCH ?= riscv64

all: $(SUB_DIRS)

$(SUB_DIRS): FORCE
	make ARCH=$(ARCH) -C $@ -B

FORCE:

.PHONY: all SUB_DIRS FORCE
