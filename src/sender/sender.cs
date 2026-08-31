// xywdl_sender — 校园网认证请求发送器 (C# 备用发送层)
//
// 用途:
//   xywdl.ps1 用 Invoke-WebRequest 发送失败时的第二层保底。
//   从 stdin 读入完整 quickauth.do URL (已 URL 编码), 发 HTTP GET,
//   把响应体原样输出到 stdout, 由主脚本判定认证结果。
//
// 语义:
//   exit 0   = 成功发出请求并拿到响应体 (stdout 为响应体, 即使 4xx/5xx)
//   exit 1   = 网络层失败 (连接失败 / 超时 / DNS 失败等)
//   exit 2   = 参数错误 (没有收到 URL)
//
// 特性:
//   - 仅依赖 .NET Framework 4.x (Windows 7+ 自带, 无需额外运行时)
//   - Proxy = null: 直连, 避免被系统代理 / 校园网网关拦截
//   - AllowAutoRedirect = false: 不跟随 302, 与 PS 端 -MaximumRedirection 0 一致
//   - 超时 15 秒, 防止卡死
//
// 构建:
//   "%SystemRoot%\Microsoft.NET\Framework64\v4.0.30319\csc.exe" /nologo /optimize+ /target:exe /out:xywdl_sender.exe sender.cs
//
// 用法 (由脚本调用, 不直接给用户用):
//   echo "http://.../quickauth.do?userid=..." | xywdl_sender.exe
//   或: xywdl_sender.exe "http://.../quickauth.do?userid=..."

using System;
using System.IO;
using System.Net;
using System.Text;

class XywdlSender
{
    static int Main(string[] args)
    {
        string url;
        if (args.Length > 0)
        {
            url = args[0].Trim();
        }
        else
        {
            // 从 stdin 读完整 URL (避免明文密码出现在进程命令行里)
            url = Console.In.ReadToEnd().Trim();
        }

        if (string.IsNullOrEmpty(url))
        {
            Console.Error.WriteLine("[sender] 未提供 URL");
            return 2;
        }

        // 剥掉可能的 BOM (U+FEFF): PS 管道用带 BOM 的 UTF8 编码传字符串时,
        // 原生进程会收到开头的 BOM 字符, 不剥掉会导致 WebRequest 报"URI 方案无效"
        url = url.Trim().TrimStart('\uFEFF');

        try
        {
            HttpWebRequest req = (HttpWebRequest)WebRequest.Create(url);
            req.Method = "GET";
            req.Proxy = null;                 // 直连, 绕开系统代理
            req.Timeout = 15000;              // 连接超时 (毫秒)
            req.ReadWriteTimeout = 15000;     // 读响应超时 (毫秒)
            req.AllowAutoRedirect = false;    // 不跟随重定向, 看原始响应
            req.UserAgent = "XXGCXY-CampusNet-AutoLogin/2.0";
            req.Accept = "text/html,application/json,*/*";

            HttpWebResponse resp = (HttpWebResponse)req.GetResponse();
            using (resp)
            {
                using (StreamReader sr = new StreamReader(resp.GetResponseStream(), Encoding.UTF8))
                {
                    string body = sr.ReadToEnd();
                    Console.Write(body);
                }
                return 0;
            }
        }
        catch (WebException we)
        {
            // 4xx/5xx (甚至 3xx 被禁后) 会走这里, 但能拿到响应对象就读取 body,
            // 交由主脚本判定; 只有连响应对象都拿不到才是网络层失败。
            HttpWebResponse resp = we.Response as HttpWebResponse;
            if (resp != null)
            {
                using (resp)
                using (StreamReader sr = new StreamReader(resp.GetResponseStream(), Encoding.UTF8))
                {
                    string body = sr.ReadToEnd();
                    Console.Write(body);
                }
                return 0;
            }
            Console.Error.WriteLine("[sender] 请求失败: " + we.Message);
            return 1;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine("[sender] 错误: " + ex.Message);
            return 1;
        }
    }
}
