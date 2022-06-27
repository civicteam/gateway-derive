import { DerivedPassService } from "../../dist/src";
import {AnchorProvider, web3} from "@project-serum/anchor";

// Civic Uniqueness Pass
const constituentPass = new web3.PublicKey("uniqobk8oGh4XBLMqM68K8M2zNu3CdYX7q5go7whQiv");

(async () => {
  const provider = AnchorProvider.env();
  const service = await DerivedPassService.build(provider);

  const [_, derivedPass] = await service.derivePass([
    constituentPass
  ], {
    expireOnUse: true,
    refreshDisabled: true,
  });

  console.log("Authority: " + provider.wallet.publicKey.toBase58());
  console.log("Derived Pass: " + derivedPass.toBase58());
})().catch(console.error);
