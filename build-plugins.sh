#!/bin/bash

# 构建后脚本：复制插件可执行文件和配置文件到目标目录

set -e

# 获取脚本所在目录
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
cd "$SCRIPT_DIR"

# 确定构建模式（release 或 debug）
PROFILE="${PROFILE:-release}"
if [ "$PROFILE" != "release" ] && [ "$PROFILE" != "debug" ]; then
    PROFILE="release"
fi

# 目标目录（构建产物在 target，最终发布内容在 output）
TARGET_DIR="$SCRIPT_DIR/target/$PROFILE"
OUTPUT_DIR="$SCRIPT_DIR/output"
PLUGINS_DIR="$OUTPUT_DIR/plugins"

# 创建 output/plugins 目录
mkdir -p "$PLUGINS_DIR"

# 插件可执行文件名（根据平台）
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" || "$OSTYPE" == "cygwin" ]]; then
    PLUGIN_EXE_SUFFIX=".exe"
else
    PLUGIN_EXE_SUFFIX=""
fi

# 复制所有插件的可执行文件或源文件
echo "Copying plugin files to $PLUGINS_DIR..."

for plugin_dir in plugins/*/; do
    if [ ! -d "$plugin_dir" ]; then
        continue
    fi
    
    plugin_name=$(basename "$plugin_dir")
    if [ "$plugin_name" = "plugin_sdk" ] || [ "$plugin_name" = "sitter" ]; then
        echo "  Skipping internal crate: $plugin_name"
        continue
    fi
    plugin_desc_file="$plugin_dir/desc.json"
    
    # 检查是否是 Python 插件
    is_python_plugin=false
    if [ -f "$plugin_desc_file" ]; then
        # 检查配置文件中 exe_path 是否为 python 或 python3
        if grep -q '"exe_path".*"python' "$plugin_desc_file" 2>/dev/null; then
            is_python_plugin=true
        fi
    fi
    
    if [ "$is_python_plugin" = true ]; then
        # Python 插件：复制整个插件目录
        plugin_dst_dir="$PLUGINS_DIR/$plugin_name"
        mkdir -p "$plugin_dst_dir"
        
        # 复制所有 Python 文件和其他必要文件
        # 复制 .py 文件
        for file in "$plugin_dir"/*.py; do
            if [ -f "$file" ]; then
                filename=$(basename "$file")
                cp "$file" "$plugin_dst_dir/$filename"
                # 确保 Python 文件有执行权限
                chmod +x "$plugin_dst_dir/$filename"
            fi
        done
        # 复制其他文件（.json, .md 等，但不包括 .example 文件）
        for file in "$plugin_dir"/*.json "$plugin_dir"/*.md; do
            if [ -f "$file" ] && [[ "$file" != *.example ]]; then
                filename=$(basename "$file")
                cp "$file" "$plugin_dst_dir/$filename"
            fi
        done
        
        # 处理 desc.json：复制到目标目录
        if [ -f "$plugin_desc_file" ]; then
            plugin_desc_dst="$plugin_dst_dir/desc.json"
            cp "$plugin_desc_file" "$plugin_desc_dst"
            echo "  ✓ Created desc.json for Python plugin: $plugin_name"
        fi
        
        echo "  ✓ Copied Python plugin: $plugin_name"
    else
        # Rust 插件：先构建插件
        echo "  Building Rust plugin: $plugin_name"
        if [ "$PROFILE" = "release" ]; then
            (cd "$plugin_dir" && cargo build --release)
        else
            (cd "$plugin_dir" && cargo build)
        fi
        
        # 创建插件目录并复制可执行文件
        plugin_dst_dir="$PLUGINS_DIR/$plugin_name"
        mkdir -p "$plugin_dst_dir"
        
        plugin_exe="$plugin_name$PLUGIN_EXE_SUFFIX"
        plugin_src="$TARGET_DIR/$plugin_exe"
        plugin_dst="$plugin_dst_dir/$plugin_exe"
        
        if [ -f "$plugin_src" ]; then
            cp "$plugin_src" "$plugin_dst"
            # 确保可执行文件有执行权限
            chmod +x "$plugin_dst"
            echo "  ✓ Copied $plugin_exe (with execute permission)"
        else
            echo "  ⚠ Plugin executable not found: $plugin_src"
        fi
        
        # 处理 desc.json：复制到目标目录
        if [ -f "$plugin_desc_file" ]; then
            plugin_desc_dst="$plugin_dst_dir/desc.json"
            cp "$plugin_desc_file" "$plugin_desc_dst"
            echo "  ✓ Created desc.json for Rust plugin: $plugin_name"
        fi

        # 复制 markdown 资源文件（如 prompt 模板）
        for file in "$plugin_dir"/*.md; do
            if [ -f "$file" ] && [[ "$file" != *.example ]]; then
                filename=$(basename "$file")
                cp "$file" "$plugin_dst_dir/$filename"
                echo "  ✓ Copied markdown resource: $filename"
            fi
        done
    fi
done

# 如果是 release 构建，则将主程序也复制到 output 目录
if [ "$PROFILE" = "release" ]; then
    if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" || "$OSTYPE" == "cygwin" ]]; then
        MAIN_EXE="egscribe.exe"
    else
        MAIN_EXE="egscribe"
    fi

    MAIN_SRC="$TARGET_DIR/$MAIN_EXE"
    MAIN_DST="$OUTPUT_DIR/$MAIN_EXE"

    mkdir -p "$OUTPUT_DIR"

    if [ -f "$MAIN_SRC" ]; then
        cp "$MAIN_SRC" "$MAIN_DST"
        chmod +x "$MAIN_DST" || true
        echo "Copied main binary to $MAIN_DST"
    else
        echo "⚠ Main executable not found: $MAIN_SRC"
    fi
fi

echo "Done!"

