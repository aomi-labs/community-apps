import type { Metadata } from "next";
import { Instrument_Serif } from "next/font/google";
import { Providers } from "./providers";
import "./globals.css";

const instrumentSerif = Instrument_Serif({
  subsets: ["latin"],
  weight: "400",
  style: ["normal", "italic"],
  display: "swap",
  variable: "--font-instrument",
});

export const metadata: Metadata = {
  title: "Aomi × Monad — One Shot",
  description:
    "One prompt, one Monad action. An Aomi sample app that turns a sentence into a signed transaction on Monad testnet.",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" className={instrumentSerif.variable}>
      <body>
        <Providers>{children}</Providers>
      </body>
    </html>
  );
}
