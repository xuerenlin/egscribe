.PHONY: build debug release clean test check help

# 平台相关命令
ifeq ($(OS),Windows_NT)
RUN_BUILD_PLUGINS_DEBUG = powershell -NoProfile -ExecutionPolicy Bypass -File build-plugins.ps1 -Profile debug
RUN_BUILD_PLUGINS_RELEASE = powershell -NoProfile -ExecutionPolicy Bypass -File build-plugins.ps1 -Profile release
else
RUN_BUILD_PLUGINS_DEBUG = PROFILE=debug ./build-plugins.sh
RUN_BUILD_PLUGINS_RELEASE = PROFILE=release ./build-plugins.sh
endif

# 默认构建debug版本
build: debug

# 构建debug版本
debug:
	@echo "Building debug version..."
	cargo build
	@echo "Copying plugins..."
	$(RUN_BUILD_PLUGINS_DEBUG)

# 构建release版本
release:
	@echo "Building release version..."
	cargo build --release
	@echo "Copying plugins..."
	$(RUN_BUILD_PLUGINS_RELEASE)

# 清理构建产物
clean:
	cargo clean

# 运行测试
test:
	cargo test

# 检查代码
check:
	cargo check

# 显示帮助信息
help:
	@echo "Available targets:"
	@echo "  build          - Build debug version (default)"
	@echo "  debug          - Build debug version"
	@echo "  release        - Build release version"
	@echo "  clean          - Clean build artifacts"
	@echo "  test           - Run tests"
	@echo "  check          - Check code without building"
	@echo "  help           - Show this help message"
	@echo "  (Windows uses build-plugins.ps1 automatically)"

