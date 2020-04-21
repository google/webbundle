CC = clang
TARGET_DIR = ../../target
BINDGEN_DIR = $(TARGET_DIR)/webbundle-ffi-bindgen
WEBBUNDLE_LIBRARY = $(TARGET_DIR)/debug/libwebbundle_ffi.a
WEBBUNDLE_HEADER = $(BINDGEN_DIR)/webbundle-ffi.h

ifeq ($(shell uname),Darwin)
    LDFLAGS := -Wl,-dead_strip
else
    LDFLAGS := -Wl,--gc-sections -lpthread -ldl
endif

all: $(BINDGEN_DIR)/main

$(BINDGEN_DIR)/main: $(BINDGEN_DIR)/main.o $(WEBBUNDLE_LIBRARY)
	$(CC) -o $@ $^ $(LDFLAGS)

$(BINDGEN_DIR)/main.o: main.c $(WEBBUNDLE_HEADER)
	$(CC) -o $@ -c $< -I $(BINDGEN_DIR)

$(WEBBUNDLE_HEADER) $(WEBBUNDLE_LIBRARY):  ../src/lib.rs ../Cargo.toml
	cargo build

clean:
	rm -rf $(BINDGEN_DIR)
