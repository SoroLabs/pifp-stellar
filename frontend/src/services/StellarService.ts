import { 
    TransactionBuilder, 
    Networks, 
    Keypair, 
    Operation, 
    Asset, 
    Account,
    Transaction
} from '@stellar/stellar-sdk';

export class StellarService {
    static async constructAtomicSwap(
        userA: string,
        assetA: string,
        amountA: string,
        userB: string,
        assetB: string,
        amountB: string,
        sequenceNumber: string
    ): Promise<string> {
        const account = new Account(userA, sequenceNumber);
        const transaction = new TransactionBuilder(account, {
            fee: '1000',
            networkPassphrase: Networks.TESTNET
        })
        .addOperation(Operation.payment({
            destination: userB,
            asset: assetA === 'native' ? Asset.native() : new Asset(assetA.split(':')[0], assetA.split(':')[1]),
            amount: amountA,
            source: userA
        }))
        .addOperation(Operation.payment({
            destination: userA,
            asset: assetB === 'native' ? Asset.native() : new Asset(assetB.split(':')[0], assetB.split(':')[1]),
            amount: amountB,
            source: userB
        }))
        .setTimeout(0)
        .build();

        return transaction.toXDR();
    }

    static signTransaction(xdr: string, secretKey: string): string {
        const transaction = new Transaction(xdr, Networks.TESTNET);
        const keypair = Keypair.fromSecret(secretKey);
        transaction.sign(keypair);
        return transaction.toXDR();
    }
}
