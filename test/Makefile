# Makefile for SHA-256 Verilator simulation

# Verilator configuration
VERILATOR = verilator
VERILATOR_FLAGS = -Wall -Wno-lint -cc --trace --exe --build -j 4

# Source files
VERILOG_SOURCES = ../top.v
CPP_SOURCES = testbench.cpp

# Output directory and executable
OBJ_DIR = obj_dir
TARGET = $(OBJ_DIR)/Vtop

# Default target
all: $(TARGET)

# Build the simulation
$(TARGET): $(VERILOG_SOURCES) $(CPP_SOURCES)
	$(VERILATOR) $(VERILATOR_FLAGS) --top-module top $(VERILOG_SOURCES) $(CPP_SOURCES)

# Run the simulation
run: $(TARGET)
	./$(TARGET)

# View waveforms (requires GTKWave)
waves: sha256_trace.vcd
	gtkwave sha256_trace.vcd &

# Clean build artifacts
clean:
	rm -rf $(OBJ_DIR) sha256_trace.vcd

# Lint only (no build)
lint:
	$(VERILATOR) -Wall --lint-only --top-module top $(VERILOG_SOURCES)

# Help target
help:
	@echo "Available targets:"
	@echo "  all    - Build the simulation"
	@echo "  run    - Build and run the simulation"
	@echo "  waves  - Open waveform viewer (requires gtkwave)"
	@echo "  clean  - Remove build artifacts"
	@echo "  lint   - Lint Verilog code only"
	@echo "  help   - Show this help"

.PHONY: all run waves clean lint help