# fightgame

To run on mac os 

```
brew install llvm
LLVM_PATH=$(brew --prefix llvm)
AR="${LLVM_PATH}/bin/llvm-ar" CC="${LLVM_PATH}/bin/clang" trunk serve --public-url /  --port=1334
```

fish shell 
```
set LLVM_PATH $(brew --prefix llvm)
AR="$LLVM_PATH/bin/llvm-ar" CC="$LLVM_PATH/bin/clang" trunk serve --public-url / --port=1334
```

Build 
```
AR="$LLVM_PATH/bin/llvm-ar" CC="$LLVM_PATH/bin/clang" trunk build --release
```
