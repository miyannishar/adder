# =============================================================================
# Adder Makefile (macOS version)
# =============================================================================
# Same pipeline as Linux, but uses macho64 object format for macOS.
# On Apple Silicon Macs, x86-64 code runs via Rosetta 2 translation.
# =============================================================================

test/%.s: test/%.snek src/main.rs
	cargo run -- $< test/$*.s

test/%.run: test/%.s runtime/start.rs
	nasm -f macho64 test/$*.s -o runtime/our_code.o
	ar rcs runtime/libour_code.a runtime/our_code.o
	rustc --target x86_64-apple-darwin -L runtime/ runtime/start.rs -o test/$*.run

clean:
	rm -f test/*.s test/*.run runtime/*.o runtime/*.a