#!/bin/bash
# Launch the clean client (HeadlessMC 2.10.0 + -lwjgl stub, our mod only). Spec: specs/plans/VANILLA_DIFF_PLAN.md VD0
D="$(cd "$(dirname "$0")" && pwd)"
cd "$D/hmc"
exec ${VD_JAVA:-/Library/Java/JavaVirtualMachines/jdk-21.jdk/Contents/Home/bin/java} -Dhmc.offline.username=${VD_USER:-VDClient} \
  "-Dhmc.jvmargs=-Xms256M -Xmx512M -XX:+UseSerialGC -Xss512K -Dio.netty.allocator.type=unpooled ${VD_JVM:-}" \
  -jar headlessmc-launcher-wrapper-2.10.0.jar --command launch fabric-loader-0.20.1-1.21.11 -lwjgl -offline
