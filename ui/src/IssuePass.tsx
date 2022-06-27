import {FC, useCallback, useMemo, useState} from "react";
import logo from './civic-logo-2022.png';
import { DerivedPassService } from "@civic/solana-derived-pass";
import {useConnection, useWallet} from "@solana/wallet-adapter-react";
import {AnchorProvider, Wallet} from "@project-serum/anchor";
import {PublicKey} from "@solana/web3.js";
import {WalletDisconnectButton, WalletMultiButton} from "@solana/wallet-adapter-react-ui";

const handleError = (error: Error):string => {
  const match = error.message.match(/Error Message: (.*)/);
  return match? match[1] : error.message;
}

export const IssuePass:FC = () => {
  const [done, setDone] = useState(false);
  const [error, setError] = useState('');
  const wallet = useWallet();
  const { connection } = useConnection();

  const authority = useMemo(() => process.env.REACT_APP_DERIVED_PASS_AUTHORITY && new PublicKey(process.env.REACT_APP_DERIVED_PASS_AUTHORITY), []);
  const derivedPass = useMemo(() => process.env.REACT_APP_DERIVED_PASS && new PublicKey(process.env.REACT_APP_DERIVED_PASS), []);

  const getTheThing = useCallback(async () => {
    if (!wallet || !wallet.publicKey || !connection || !authority || !derivedPass) return;

    const provider = new AnchorProvider(
      connection,
      wallet as unknown as Wallet,
      AnchorProvider.defaultOptions()
    );

    const service = await DerivedPassService.build(provider)
    service.issue(authority, derivedPass).then(() => setDone(true)).catch(e => setError(handleError(e)));

  }, [wallet, connection, authority, derivedPass, setError])

  if (!authority || !derivedPass) return <div>Error: Missing environment variables. Check the .env file</div>

  return <>
    {!wallet?.connected && <WalletMultiButton /> }
    {wallet?.connected && <WalletDisconnectButton />}
    <button onClick={getTheThing} disabled={!wallet || !wallet.publicKey || !connection} className="wallet-adapter-button wallet-adapter-button-trigger">
      <img src={logo} className="wallet-adapter-button-start-icon" alt="logo"/>
      Get the thing...
    </button>
    { done ? <div>Done!</div> : null }
    { error ? <div>Error: {error}</div> : null }
  </>
}
