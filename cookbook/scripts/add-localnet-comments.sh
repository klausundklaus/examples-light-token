#!/bin/bash
# Add localnet/devnet comments to action files
# Run from: /home/tilo/Workspace/examples-light-token/cookbook/actions/

cd /home/tilo/Workspace/examples-light-token/cookbook/actions/

for file in *.ts; do
    echo "Processing $file..."

    # Add "// devnet:" before RPC_URL line
    sed -i 's/^const RPC_URL = /\/\/ devnet:\nconst RPC_URL = /' "$file"

    # Add localnet comment after RPC_URL line
    sed -i '/^const RPC_URL = .*$/a \/\/ localnet:\n\/\/ const RPC_URL = undefined;' "$file"

    # Add comments around createRpc(RPC_URL)
    sed -i 's/const rpc = createRpc(RPC_URL);/\/\/ devnet:\n    const rpc = createRpc(RPC_URL);\n    \/\/ localnet:\n    \/\/ const rpc = createRpc();/' "$file"
done

echo "Done!"
