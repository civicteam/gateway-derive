import {useConnection, useWallet} from "@solana/wallet-adapter-react";
import {useEffect, useState} from "react";
import { DerivedPassService } from "@civic/solana-derived-pass";
import {AnchorProvider, Wallet} from "@project-serum/anchor";

export const useDerivedPass = () => {
  const wallet = useWallet();
  const { connection } = useConnection();
  const [service, setService] = useState<DerivedPassService>()


  useEffect(() => {
    if (!wallet || !wallet.publicKey || !connection) return;

    const provider = new AnchorProvider(
      connection,
      wallet as unknown as Wallet,
      AnchorProvider.defaultOptions()
    );

    DerivedPassService.build(provider).then(setService)
  }, [wallet, connection]);

  return service;
}
