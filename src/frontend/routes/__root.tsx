import { createRootRoute } from '@tanstack/react-router';

import alloyLogo from '../assets/alloy.png';
import icLogo from '../assets/ic.svg';

import { backend } from '../../backend/declarations';
import { useState, useEffect } from 'react';
import { ethers } from 'ethers';

export const Route = createRootRoute({
  component: Root,
});

declare global {
  interface Window {
    ethereum: any;
  }
}

function Root() {
  const [address, setAddress] = useState('');
  const [walletAddress, setWalletAddress] = useState('');
  const [isApproving, setIsApproving] = useState(false);

  const injectedProvider = new ethers.BrowserProvider(window.ethereum); // For MetaMask

  const abi = [
    'function approve(address spender, uint256 amount) external returns (bool)',
    //"function getSubmittedNames() public view returns (string[])"
  ];

  console.log(abi);

  console.log(injectedProvider);

  const addressResult = async () => {
    const result = await backend.get_address();
    if ('Ok' in result) {
      setAddress(result.Ok);
    } else {
      console.error('Failed to get address:', result.Err);
    }
  };

  const getWalletAddress = async () => {
    try {
      const accounts = await window.ethereum.request({
        method: 'eth_requestAccounts',
      });
      setWalletAddress(accounts[0]);
    } catch (error) {
      console.error('Failed to get wallet address:', error);
    }
  };

  const approveTransfer = async () => {
    try {
      //const signer = await injectedProvider.getSigner();
      const provider = new ethers.BrowserProvider(window.ethereum);
      setIsApproving(true);
      const signer = await provider.getSigner();

      const contract = new ethers.Contract(
        '0x1c7D4B196Cb0C7B01d743Fbc6116a902379C7238',
        abi,
        signer,
      );
      const tx = await contract.approve(address, ethers.parseUnits('10.0', 6));
      await tx.wait();
      console.log('Transfer approved');
      setIsApproving(false);
    } catch (error) {
      console.error('Failed to approve transfer:', error);
      setIsApproving(false);
    }
  };

  const transferUsdc = async () => {
    try {
      setIsApproving(true);
      const result = await backend.transfer_usdc();
      console.log('Transfer result:', result);
      setIsApproving(false);
    } catch (error) {
      setIsApproving(false);
      console.error('Failed to transfer USDC:', error);
    }
  };

  useEffect(() => {
    addressResult();
    getWalletAddress();
  }, []);

  return (
    <main>
      <div>
        <a href="https://alloy.rs" target="_blank" rel="noreferrer">
          <img src={alloyLogo} className="logo" alt="Vite logo" />
        </a>
        <a href="https://internetcomputer.org" target="_blank" rel="noreferrer">
          <img src={icLogo} className="logo" alt="React logo" />
        </a>
      </div>
      <h1>Alloy + ICP</h1>
      <p>
        {address ? `The backend canister address is: ${address}` : 'Loading...'}
      </p>
      <p>
        {walletAddress
          ? `The Injected provider Wallet Address is: ${walletAddress}`
          : 'Loading...'}
      </p>
      <button disabled={isApproving} onClick={approveTransfer}>
        Approve Transfer
      </button>

      <button disabled={isApproving} onClick={transferUsdc}>
        Transfer USDC
      </button>
    </main>
  );
}
