import { ReactNode } from 'react';

interface SectionProps {
  name: string;
  children: ReactNode;
}

export default function Section({ name, children }: SectionProps) {
  return (
    <div className="section">
      <h2 className="text-lg font-semibold text-gray-900 mb-4">{name}</h2>
      {children}
    </div>
  );
}
