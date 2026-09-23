#!/bin/bash
# 构建 mc-console APK —— 纯 Android SDK 工具链，不需要 Gradle。
#
# 跨平台：原脚本写死了 `d8.bat` / `apksigner.bat`，在 Linux/macOS 上直接
# "No such file or directory"（CI 跑的就是 ubuntu）。现在按平台选后缀：
#   aapt2 / zipalign  -> Windows 上是 .exe（原生程序）
#   d8 / apksigner    -> Windows 上是 .bat（Java 包装脚本）
#
# 版本也不再写死：原来 BT 固定 build-tools/36.0.0、PLAT 固定 platforms/android-36，
# SDK 一升级就失效。改为取已安装的最高版本，可用环境变量覆盖。
set -euo pipefail

SDK=${ANDROID_SDK_ROOT:-${ANDROID_HOME:-$HOME/Android/Sdk}}
JDK=${JAVA_HOME:-/usr/lib/jvm/temurin-17-jdk}/bin

SUF_EXE=""
SUF_BAT=""
case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*) SUF_EXE=".exe"; SUF_BAT=".bat" ;;
esac

BT=${LYCO_BUILD_TOOLS:-$(ls -d "$SDK"/build-tools/* 2>/dev/null | sort -V | tail -1)}
PLAT_JAR=${LYCO_PLATFORM_JAR:-$(ls -d "$SDK"/platforms/android-*/android.jar 2>/dev/null | sort -V | tail -1)}

if [ ! -d "${BT:-}" ]; then
  echo "找不到 build-tools（SDK=${SDK}）。装一个 build-tools 或用 LYCO_BUILD_TOOLS 指定。" >&2
  exit 1
fi
if [ ! -f "${PLAT_JAR:-}" ]; then
  echo "找不到 platforms/android-*/android.jar（SDK=${SDK}）。装一个 platform 或用 LYCO_PLATFORM_JAR 指定。" >&2
  exit 1
fi
if [ ! -x "$BT/d8$SUF_BAT" ]; then
  echo "缺少 $BT/d8$SUF_BAT —— build-tools 不完整。" >&2
  exit 1
fi

cd "$(dirname "$0")"
rm -rf out
mkdir -p out/classes out/dex

echo "[1/5] aapt2 link ... (build-tools ${BT##*/})"
"$BT/aapt2$SUF_EXE" link -o out/base.apk -I "$PLAT_JAR" \
  --manifest AndroidManifest.xml --java out

R_JAVA=$(find out -name R.java | head -1)
if [ -z "$R_JAVA" ]; then
  echo "aapt2 没有生成 R.java —— 检查 AndroidManifest.xml 的 package 属性。" >&2
  exit 1
fi
PKG_DIR=$(dirname "${R_JAVA#out/}")     # 例: local/mc/console
echo "      包名: ${PKG_DIR//\//.}"

echo "[2/5] javac ..."
"$JDK/javac" --release 8 -nowarn -classpath "$PLAT_JAR" -d out/classes \
  "java/${PKG_DIR}/MainActivity.java" "$R_JAVA"

echo "[3/5] d8 dex ..."
"$BT/d8$SUF_BAT" --release --lib "$PLAT_JAR" --output out/dex "out/classes/${PKG_DIR}"/*.class

echo "[4/5] 把 classes.dex 塞进 apk ..."
( cd out/dex && zip -q ../base.apk classes.dex )

echo "[5/5] zipalign + 签名 ..."
"$BT/zipalign$SUF_EXE" -f 4 out/base.apk out/aligned.apk
if [ ! -f out/debug.keystore ]; then
  "$JDK/keytool" -genkeypair -keystore out/debug.keystore -storepass android \
    -alias androiddebugkey -keypass android \
    -dname "CN=Android Debug,O=Android,C=US" -keyalg RSA -keysize 2048 -validity 10000
fi
"$BT/apksigner$SUF_BAT" sign --ks out/debug.keystore --ks-pass pass:android \
  --key-pass pass:android --out out/mc-console.apk out/aligned.apk

echo "APK_OK: out/mc-console.apk"
ls -la out/mc-console.apk
