SIGN_IDENTITY = "Apple Development: Bastiaan Van der Hoek (576HK37AL8)"

ID3.xcframework: target/libid3_macos.a
	xcodebuild -create-xcframework \
      -library target/libid3_macos.a \
      -headers ./include/ \
      -output ID3.xcframework
	zip -r bundle.zip ID3.xcframework
	openssl dgst -sha256 bundle.zip

define lipo_sign
    echo $2
	lipo -create -output $2 $1
	codesign -s $(SIGN_IDENTITY) -f $2
endef

target/libid3_macos.a: target/aarch64-apple-darwin/release/libid3rs.a target/x86_64-apple-darwin/release/libid3rs.a
	$(call lipo_sign,$^,$@)

target/x86_64-apple-darwin/release/libid3rs.a: include/id3.h
	cargo build --release --target x86_64-apple-darwin

target/aarch64-apple-darwin/release/libid3rs.a: include/id3.h
	cargo build --release --target aarch64-apple-darwin

include/id3.h: src/lib.rs
	cbindgen --lang c --output include/id3.h

clean:
	rm -rf ID3.xcframework
	rm include/id3.h
	cargo clean
