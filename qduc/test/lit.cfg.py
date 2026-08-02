import os

import lit.formats

config.name = "qduc"
config.test_format = lit.formats.ShTest(True)
config.suffixes = [".mlir"]

config.test_source_root = config.qduc_test_source_root
config.test_exec_root = config.qduc_test_binary_root

# RUN lines invoke `qduc-opt`/`FileCheck` by plain name; put their build
# output directories on PATH instead of using lit substitutions.
tool_dirs = {os.path.dirname(config.qduc_opt), os.path.dirname(config.filecheck)}
config.environment["PATH"] = os.pathsep.join(
    [*tool_dirs, config.environment.get("PATH", "")]
)
