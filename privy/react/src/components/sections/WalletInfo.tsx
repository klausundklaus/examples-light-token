import Section from '../reusables/Section';
import CopyButton from '../reusables/CopyButton';

interface WalletInfoProps {
  address: string;
}

export default function WalletInfo({ address }: WalletInfoProps) {
  if (!address) return null;

  return (
    <Section name="Wallet">
      <div className="flex items-center justify-between">
        <div>
          <p className="text-sm text-gray-500">Address</p>
          <p className="font-mono text-sm">
            {address.slice(0, 12)}...{address.slice(-12)}
          </p>
        </div>
        <CopyButton text={address} label="Address" />
      </div>
    </Section>
  );
}
