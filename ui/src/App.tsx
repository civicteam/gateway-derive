import React, {useMemo} from 'react';
import logo from './civic-logo-2022.png';

import { IssuePass } from './IssuePass';
import {WalletAdapterNetwork} from "@solana/wallet-adapter-base";
import {
  GlowWalletAdapter, PhantomWalletAdapter,
  SlopeWalletAdapter,
  SolflareWalletAdapter,
  SolletWalletAdapter, TorusWalletAdapter
} from "@solana/wallet-adapter-wallets";
import {ConnectionProvider, WalletProvider} from "@solana/wallet-adapter-react";
import {clusterApiUrl} from "@solana/web3.js";
import {WalletModalProvider} from "@solana/wallet-adapter-react-ui";

import '@solana/wallet-adapter-react-ui/styles.css';
import './App.css';

function App() {
  const network = WalletAdapterNetwork.Devnet;
  const endpoint = useMemo(() => clusterApiUrl(network), [network]);
  const wallets = useMemo(
    () => [
      new PhantomWalletAdapter(),
      new GlowWalletAdapter(),
      new SlopeWalletAdapter(),
      new SolflareWalletAdapter({ network }),
      new TorusWalletAdapter(),
      new SolletWalletAdapter({ network }),
    ],
    [network]
  );

  return (
    <ConnectionProvider endpoint={endpoint}>
      <WalletProvider wallets={wallets} autoConnect>
        <WalletModalProvider>
          <div className="App">
            <header className="App-header">
              <img src={logo} className="App-logo" alt="logo" />
              <IssuePass/>
            </header>
          </div>
        </WalletModalProvider>
      </WalletProvider>
    </ConnectionProvider>
  );
}

export default App;
