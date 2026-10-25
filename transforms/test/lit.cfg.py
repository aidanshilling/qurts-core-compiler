import os

import lit.formats

config.name = "transforms"
config.test_format = lit.formats.ShTest(True)
config.suffixes = [".mlir"]

config.test_source_root = config.qurts_test_source_root
config.test_exec_root = config.qurts_test_binary_root

tool_dirs = {os.path.dirname(config.qurts_opt), os.path.dirname(config.filecheck)}
config.environment["PATH"] = os.pathsep.join(
    [*tool_dirs, config.environment.get("PATH", "")]
)
