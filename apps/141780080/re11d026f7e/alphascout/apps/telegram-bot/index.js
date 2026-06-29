require("dotenv").config();

const { Telegraf } = require("telegraf");
const axios = require("axios");

const bot = new Telegraf(process.env.TELEGRAM_BOT_TOKEN);

const BACKEND_URL = process.env.BACKEND_URL || "http://127.0.0.1:8080";

// 🚀 Start command
bot.start((ctx) => {
  ctx.reply(
    "🚀 Welcome to AlphaScout\n\nAI market intelligence agent for Base traders."
  );
});

// 🧠 Main handler (ONLY ONE)
bot.on("text", async (ctx) => {
  try {
    const userMessage = ctx.message.text;

    const response = await axios.post(`${BACKEND_URL}/chat`, {
      message: userMessage,
    });

    const reply =
      response?.data?.reply ||
      response?.data?.message ||
      "No response from AlphaScout.";

    await ctx.reply(reply);
  } catch (err) {
    console.error("Error:", err.message);
    await ctx.reply("⚠️ AlphaScout backend unavailable.");
  }
});

// 🚀 Launch bot
bot.launch();

console.log("🤖 AlphaScout Telegram bot running...");
