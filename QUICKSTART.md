# ConvexFX Delta - Quick Start Guide

Get up and running in 5 minutes! 🚀

## 1. Start the Server

```bash
./start_web_app.sh
```

**Expected Output:**
```
✅ Server started successfully!
📊 Web interface: http://localhost:8080
```

## 2. Open Your Browser

Navigate to: **http://localhost:8080**

## 3. Create a User (Optional)

Demo users `alice`, `bob`, and `charlie` are pre-configured.

To create your own:
1. Click the **Portfolio** tab
2. Enter a username
3. Click **Register User**

## 4. View a Balance

1. Stay in the **Portfolio** tab
2. Select `alice` from the dropdown
3. Click **Load Balance**

You'll see:
- USD: 10,000
- EUR: 5,000
- JPY: 1,000,000

## 5. Execute a Trade

1. Go to the **Exchange** tab
2. In the Trade section:
   - Enter amount: `1000`
   - From: `USD`
   - To: `EUR`
3. Click **Execute Trade**

Done! You've just completed your first trade. 🎉

## 6. Transfer Tokens

1. In the **Exchange** tab, scroll to Transfers
2. Fill in:
   - From User: `alice`
   - To User: `bob`
   - Asset: `USD`
   - Amount: `100`
3. Click **Send Transfer**

## 7. View Documentation

Click the **Documentation** tab for complete instructions.

## Stopping the Server

```bash
./stop_web_app.sh
```

Or press `Ctrl+C` if running in foreground.

---

## Quick Tips

💡 **The server runs continuously** - this is normal! It's waiting for requests.

💡 **Check system status** - Click the "System Status" button in the nav bar

💡 **Watch for notifications** - Success/error messages appear in the top-right

💡 **Start small** - Test with smaller amounts first

💡 **Check price impact** - Lower percentages mean better prices

---

## Troubleshooting

**Port in use?**
```bash
./stop_web_app.sh
./start_web_app.sh
```

**User not found?**
- Use `alice`, `bob`, or `charlie`
- Or create a new user in Portfolio tab

**Need help?**
- Read the full guide: `USER_GUIDE.md`
- Check logs: `tail -f logs/web_app.log`
- API health: `curl http://localhost:8080/api/health`

---

## What's Next?

- 📖 Read the complete [User Guide](USER_GUIDE.md)
- 🔧 Check the [Web App README](WEB_APP_README.md)
- 💻 Try the CLI demo: `cargo run --bin simple_demo --features demo -- demo`
- 🌐 Explore the API endpoints

---

**Happy Trading!** 🎯

