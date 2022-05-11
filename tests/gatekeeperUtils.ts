import {AnchorProvider, web3} from '@project-serum/anchor';

export const fund = (provider: AnchorProvider, to: web3.PublicKey, lamports: number = 500_000_000) =>
  provider.sendAndConfirm(
    new web3.Transaction().add(
      web3.SystemProgram.transfer({
        fromPubkey: provider.wallet.publicKey,
        toPubkey: to,
        lamports,
      })
    )
  );

