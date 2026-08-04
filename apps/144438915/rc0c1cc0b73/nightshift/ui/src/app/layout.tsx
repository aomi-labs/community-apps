import type { Metadata } from "next";
import { GeistSans } from "geist/font/sans";
import { GeistMono } from "geist/font/mono";
import { PT_Serif, Source_Serif_4 } from "next/font/google";
import "./globals.css";

const ptSerif = PT_Serif({
  subsets: ["latin"],
  weight: ["400", "700"],
  variable: "--font-serif",
  display: "swap",
});
const sourceSerif = Source_Serif_4({
  subsets: ["latin"],
  weight: ["600"],
  variable: "--font-wordmark",
  display: "swap",
});

export const metadata: Metadata = {
  title: "Nightshift — a Solana agent inside a box you can read",
  description:
    "A bounded, recurring Solana agent that trades while you sleep. It quotes and stages inside a permission envelope fixed in code — and physically cannot exceed it.",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html
      lang="en"
      className={`${GeistSans.variable} ${GeistMono.variable} ${ptSerif.variable} ${sourceSerif.variable}`}
    >
      <body>{children}</body>
    </html>
  );
}
