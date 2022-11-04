import {FC, useCallback, useEffect, useMemo, useState} from "react";
import logo from './civic-logo-2022.png';
import {useConnection, useWallet} from "@solana/wallet-adapter-react";
import {PublicKey} from "@solana/web3.js";
import {WalletDisconnectButton, WalletMultiButton} from "@solana/wallet-adapter-react-ui";
import {useDerivedPass} from "./useDerivedPass";
import {
  findGatewayToken,
  GatewayToken,
} from "@identity.com/solana-gateway-ts";

const handleError = (error: Error):string => {
  const match = error.message.match(/Error Message: (.*)/);
  return match? match[1] : error.message;
}

export const IssuePass:FC = () => {
  const [token, setToken] = useState<GatewayToken>();
  const [done, setDone] = useState(false);
  const [error, setError] = useState('');
  const wallet = useWallet();
  const { connection } = useConnection();
  const service = useDerivedPass();

  const authority = useMemo(() => process.env.REACT_APP_DERIVED_PASS_AUTHORITY && new PublicKey(process.env.REACT_APP_DERIVED_PASS_AUTHORITY), []);
  const derivedPass = useMemo(() => process.env.REACT_APP_DERIVED_PASS && new PublicKey(process.env.REACT_APP_DERIVED_PASS), []);

  useEffect(() => {
    if (!wallet || !connection || !wallet.publicKey || !derivedPass) return;

    findGatewayToken(connection, wallet.publicKey, derivedPass).then(gt => gt && setToken(gt))
  }, [wallet, connection, derivedPass])

  const getTheThing = useCallback(async () => {
    if (!service || !authority || !derivedPass) return;

    service.issue(authority, derivedPass).then(() => setDone(true)).catch((e: any) => setError(handleError(e)));

  }, [service, authority, derivedPass, setError])

  if (!authority || !derivedPass) return <div>Error: Missing environment variables. Check the .env file</div>

  return <>
    {!wallet?.connected && <WalletMultiButton /> }
    {wallet?.connected && <WalletDisconnectButton />}
    <button onClick={getTheThing} disabled={!wallet || !wallet.publicKey || !connection} className="wallet-adapter-button wallet-adapter-button-trigger">
      <img src={logo} className="wallet-adapter-button-start-icon" alt="logo"/>
      Get the thing...
    </button>
    { token ? <div>You have the thing!</div> : null }
    { done ? <div>Done!</div> : null }
    { error ? <div>Error: {error}</div> : null }
  </>
}
