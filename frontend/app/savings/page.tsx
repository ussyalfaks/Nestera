import Link from 'next/link';

export default function SavingsPage() {
  return (
    <div className="p-8">
      <h1 className="text-3xl font-bold mb-4">Savings Plans</h1>
      <p className="mb-6">Explore available savings circles or create your own.</p>
      
      <nav className="flex gap-4 border-t pt-4">
        <Link href="/" className="text-blue-600 hover:underline">← Home</Link>
        <Link href="/dashboard" className="text-blue-600 hover:underline">User Dashboard →</Link>
      </nav>
    </div>
  );
}