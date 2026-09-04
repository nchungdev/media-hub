/**
 * Plex Library Grid, Filter & Season Navigator
 */
import { showToast } from '../../core/toast.js';


    const plexShowsData = [
      { id: "78864", title: "Black Jack (1993)", vn: "Bác Sĩ Quái Dị Black Jack", year: "1993", seasons: "3 Seasons", files: 183, qual: "1080p BDRip", qualClass: "1080p", type: "anime", poster: "/api/poster?tvdb=78864", backdrop: "https://media.kitsu.app/anime/cover_images/1364/large.jpg", path: "gdrive:Phim/TV Shows/Black Jack (1993) {tvdb-78864} [tvdbid-78864]", seasonList: ["Season 00 (OVA 1080p - 12 tập + 2 Movies)", "Season 01 (TV 2004 - 61 tập)", "Season 02 (Black Jack 21 - 17 tập)"], plot: "Black Jack là một bác sĩ phẫu thuật thiên tài nhưng hành nghề không có giấy phép. Ông đi khắp nơi chữa trị cho những ca bệnh nan y hiểm nghèo với mức chi phí phẫu thuật khổng lồ, nhưng luôn bảo vệ công lý và lòng nhân đạo.", studio: "Tezuka Productions", genres: ["Animation", "Drama", "Medical", "Mystery"] },
      { id: "79354", title: "The File of Young Kindaichi (1997)", vn: "Thám Tử Kindaichi (Anime 1997)", year: "1997", seasons: "5 Seasons", files: 601, qual: "480p DVD", qualClass: "480p", type: "anime", poster: "/api/poster?tvdb=79354", backdrop: "", path: "gdrive:Phim/TV Shows/The File of Young Kindaichi (1997) {tvdb-79354} [tvdbid-79354]", seasonList: ["Season 00 (Specials & Movies)", "Season 01 (Tập 1-73)", "Season 02 (Tập 74-148)", "Season 03 (SP)", "Season 04 (SP)"], plot: "Kindaichi Hajime, cháu trai của thám tử huyền thoại Kindaichi Kosuke, là một học sinh trung học vụng về nhưng sở hữu chỉ số IQ 180 và khả năng suy luận phi thường, chuyên giải mã những vụ án giết người bí ẩn và ly kỳ nhất.", studio: "Toei Animation", genres: ["Animation", "Mystery", "Detective", "Suspense"] },
      { id: "279782", title: "The File of Young Kindaichi Returns (2014)", vn: "Thám Tử Kindaichi Returns (BDRip)", year: "2014", seasons: "2 Seasons", files: 320, qual: "1080p BDRip", qualClass: "1080p", type: "anime", poster: "/api/poster?tvdb=279782", backdrop: "", path: "gdrive:Phim/TV Shows/The File of Young Kindaichi Returns (2014) {tvdb-279782} [tvdbid-279782]", seasonList: ["Season 01 (25 tập - 1080p BDRip)", "Season 02 (22 tập - 1080p BDRip)", "Featurettes (Creditless OP/ED)", "Other (Menu BDRip)"], plot: "Phần tiếp theo của loạt phim trinh thám Kindaichi với đồ họa HD hiện đại, đưa Hajime và Miyuki vào hàng loạt vụ án hóc búa, đối đầu với Đạo tặc Takato Yoichi.", studio: "Toei Animation", genres: ["Animation", "Mystery", "Detective", "Suspense"] },
      { id: "79460", title: "The Files of the Young Kindaichi (1995)", vn: "Thám Tử Kindaichi (Live Action)", year: "1995", seasons: "2 Seasons", files: 13, qual: "1080p BDRip", qualClass: "1080p", type: "live", poster: "/api/poster?tvdb=79460", backdrop: "", path: "gdrive:Phim/TV Shows/The Files of the Young Kindaichi (1995) {tvdb-79460} [tvdbid-79460]", seasonList: ["Season 00 (Movies)", "Season 03 (TV Series 1080p - Tsuyoshi Domoto)"], plot: "Series truyền hình người đóng kinh điển chuyển thể từ bộ truyện tranh Thám tử Kindaichi với sự tham gia của Tsuyoshi Domoto và Tomosaka Rie.", studio: "NIPPON TV", genres: ["Drama", "Mystery", "Crime"] },
      { id: "227501", title: "Mashin Hero Wataru (1988)", vn: "Thần Long Đấu Sĩ Wataru", year: "1988", seasons: "5 Seasons", files: 720, qual: "1080p BDRip", qualClass: "1080p", type: "anime", poster: "/api/poster?tvdb=227501", backdrop: "", path: "gdrive:Phim/TV Shows/Mashin Hero Wataru (1988) {tvdb-227501} [tvdbid-227501]", seasonList: ["Season 00 (OVAs)", "Season 01 (TV 1988)", "Season 02 (TV 1990)", "Season 03 (Chou Mashin Wataru - 51 tập)", "Season 04 (Seven Tamashii)", "Behind The Scenes & Featurettes"], plot: "Cậu bé 9 tuổi Ikusabe Wataru được đưa đến thế giới thần tiên Soukaizan, điều khiển robot Ryujinmaru (Thần Long Đấu Sĩ) chiến đấu giải cứu 7 tầng núi cầu vồng khỏi thế lực bóng tối Doakudar.", studio: "Sunrise", genres: ["Animation", "Action", "Adventure", "Mecha", "Fantasy"] },
      { id: "74599", title: "Monster (2004)", vn: "Quái Vật Monster", year: "2004", seasons: "1 Season", files: 134, qual: "1080p BluRay", qualClass: "1080p", type: "anime", poster: "/api/poster?tvdb=74599", backdrop: "https://media.kitsu.app/anime/cover_images/10/large.jpg", path: "gdrive:Phim/TV Shows/Monster (2004) {tvdb-74599} [tvdbid-74599]", seasonList: ["Season 01 (74 tập DVD 480p + 74 tập BluRay 1080p DUAL FLAC)"], plot: "Bác sĩ phẫu thuật não tài năng Kenzou Tenma đã cứu mạng một cậu bé mồ côi mang tên Johan Liebert thay vì thị trưởng thành phố. Nhiều năm sau, ông bàng hoàng nhận ra đứa trẻ mình từng cứu lại là một con quái vật tâm thần giết người hàng loạt máu lạnh.", studio: "Madhouse", genres: ["Animation", "Crime", "Drama", "Mystery", "Psychological", "Thriller"] },
      { id: "75939", title: "Battle B-Daman (2004)", vn: "Chiến Binh B-Daman", year: "2004", seasons: "4 Seasons", files: 62, qual: "1080p / 480p", qualClass: "1080p", type: "anime", poster: "/api/poster?tvdb=75939", backdrop: "", path: "gdrive:Phim/TV Shows/Battle B-Daman (2004) {tvdb-75939} [tvdbid-75939]", seasonList: ["Season 01 (52 tập)", "Season 02 (Fire Spirits - 51 tập)", "Season 03 (Cross Fight B-Daman 1080p)", "Season 04 (Cross Fight B-Daman eS)"], plot: "Cậu bé Yamato Delgado sở hữu B-Daman huyền thoại Cobalt Blade, bước vào hành trình trở thành B-Player vĩ đại nhất để bảo vệ thế giới B-Daman khỏi Shadow Alliance.", studio: "Nippon Animation", genres: ["Animation", "Action", "Adventure", "Kids"] },
      { id: "79178", title: "Transformers - Car Robots (2000)", vn: "Transformers: Car Robots", year: "2000", seasons: "1 Season", files: 44, qual: "480p DVD", qualClass: "480p", type: "anime", poster: "/api/poster?tvdb=79178", backdrop: "", path: "gdrive:Phim/TV Shows/Transformers - Car Robots (2000) {tvdb-79178} [tvdbid-79178]", seasonList: ["Season 01 (39/39 tập trọn bộ)", "Featurettes (Menu & Extras)"], plot: "Thủ lĩnh Autobot Fire Convoy (Optimus Prime) cùng đội quân Car Robots chiến đấu bảo vệ Trái Đất khỏi âm mưu xâm lăng của Gigatron và lực lượng Predacons/Decepticons.", studio: "Studio Gallop", genres: ["Animation", "Action", "Mecha", "Sci-Fi"] },
      { id: "454526", title: "WUKONG: Đại Viên Hồn (2025)", vn: "Tây Hành Kỷ: Đại Viên Hồn (WUKONG)", year: "2025", seasons: "1 Season", files: 13, qual: "1080p WEB-DL", qualClass: "1080p", type: "anime", poster: "/api/poster?tvdb=454526", backdrop: "", path: "gdrive:Phim/TV Shows/WUKONG (2025) {tvdb-454526} [tvdbid-454526]", seasonList: ["Season 01 (12/12 tập 1080p WEB-DL Tencent)"], plot: "Phần ngoại truyện chính thức của Tây Hành Kỷ về Tôn Ngộ Không, thức tỉnh sức mạnh Đại Viên Hồn tung hoành tam giới với đồ họa 3D đỉnh cao 1080p WEB-DL từ Tencent Video.", studio: "Tencent Video", genres: ["Animation", "Action", "Adventure", "Fantasy", "Mythology"] },
      { id: "350711", title: "The Westward (2018)", vn: "Tây Hành Kỷ", year: "2018", seasons: "2 Seasons", files: 23, qual: "1080p WEB-DL", qualClass: "1080p", type: "anime", poster: "/api/poster?tvdb=350711", backdrop: "", path: "gdrive:Phim/TV Shows/The Westward (2018) {tvdb-350711} [tvdbid-350711]", seasonList: ["Season 00 (Cuồng Vương - 9 tập)", "Season 04 (12 tập 1080p)"], plot: "16 năm sau khi thỉnh kinh thành công, Kỳ Kinh mang sức mạnh vô thượng bị Thiên đình tranh đoạt. Thầy trò Đường Tam Tạng một lần nữa tập hợp, mở ra hành trình Trả Kinh gian nan bảo vệ tam giới.", studio: "Tencent Penguin Pictures", genres: ["Animation", "Action", "Adventure", "Fantasy"] },
      { id: "259259", title: "Kingdom (2012)", vn: "Vương Giả Thiên Hạ", year: "2012", seasons: "6 Seasons", files: 164, qual: "1080p BDRip", qualClass: "1080p", type: "anime", poster: "/api/poster?tvdb=259259", backdrop: "https://media.kitsu.app/anime/cover_images/7696/large.jpg", path: "gdrive:Phim/TV Shows/Kingdom (2012) {tvdb-259259} [tvdbid-259259]", seasonList: ["Season 01 (38 tập)", "Season 02 (39 tập)", "Season 03 (26 tập)", "Season 04 (26 tập)", "Season 05 (13 tập)", "Season 06 (Upcoming)"], plot: "Thời Chiến Quốc Trung Hoa, cậu thiếu niên mồ côi Tín nuôi ước mơ trở thành Thiên Hạ Đại Tướng Quân, đồng hành cùng Doanh Chính trên con đường thống nhất thiên hạ.", studio: "Studio Pierrot", genres: ["Animation", "Action", "Drama", "Historical", "Military"] },
      { id: "80674", title: "Furuhata Ninzaburo (1994)", vn: "Thám Tử Cổ Điển Furuhata", year: "1994", seasons: "4 Seasons", files: 44, qual: "480p DVD", qualClass: "480p", type: "live", poster: "/api/poster?tvdb=80674", backdrop: "", path: "gdrive:Phim/TV Shows/Furuhata Ninzaburo (1994) {tvdb-80674} [tvdbid-80674]", seasonList: ["Season 00 (Toàn bộ Specials SP)", "Season 01 (12 tập)", "Season 02 (10 tập)", "Season 03 (11 tập)"], plot: "Thanh tra Furuhata Ninzaburo lịch thiệp, thông minh và dí dỏm, sử dụng tài quan sát và suy luận tâm lý sắc sảo để khiến những kẻ thủ ác hoàn hảo phải tự lộ sơ hở.", studio: "Fuji Television", genres: ["Drama", "Crime", "Mystery"] },
      { id: "320122", title: "The Three-Eyed One (1990)", vn: "Cậu Bé 3 Mắt (Mitsume ga Tooru)", year: "1990", seasons: "1 Season", files: 292, qual: "480p DVD", qualClass: "480p", type: "anime", poster: "/api/poster?tvdb=320122", backdrop: "", path: "gdrive:Phim/TV Shows/The Three-Eyed One (1990) {tvdb-320122} [tvdbid-320122]", seasonList: ["Season 01 (48/48 tập trọn bộ + Full Vietsub ASS/SRT)"], plot: "Sharaku Hosuke, hậu duệ cuối cùng của tộc Ba Mắt cổ xưa, khi được gỡ miếng băng dính trên trán sẽ thức tỉnh trí tuệ siêu phàm và ma thuật huyền bí giải mã các bí ẩn văn minh cổ đại.", studio: "Tezuka Productions", genres: ["Animation", "Adventure", "Fantasy", "Mystery", "Sci-Fi"] },
      { id: "230211", title: "Tantei Gakuen Q (2003)", vn: "Học Viện Thám Tử Q", year: "2003", seasons: "1 Season", files: 47, qual: "480p DVD", qualClass: "480p", type: "anime", poster: "/api/poster?tvdb=230211", backdrop: "https://media.kitsu.app/anime/cover_images/374/large.jpg", path: "gdrive:Phim/TV Shows/Tantei Gakuen Q (2003) {tvdb-230211} [tvdbid-230211]", seasonList: ["Season 01 (45/45 tập trọn bộ + Full Vietsub ASS)"], plot: "Kyuu cùng các bạn trẻ ưu tú tại Học viện Thám tử Dan Morihiko giải mã các vụ kỳ án, đối đầu với tổ chức tội phạm ngầm Minh vương tinh (Pluto).", studio: "Pierrot", genres: ["Animation", "Mystery", "Detective", "Shounen"] },
      { id: "335191", title: "Hakyuu Houshin Engi (2018)", vn: "Bá Khí Phong Thần Diễn Nghĩa", year: "2018", seasons: "1 Season", files: 25, qual: "1080p BDRip", qualClass: "1080p", type: "anime", poster: "/api/poster?tvdb=335191", backdrop: "", path: "gdrive:Phim/TV Shows/Hakyuu Houshin Engi (2018) {tvdb-335191} [tvdbid-335191]", seasonList: ["Season 01 (24/24 tập 1080p BDRip)"], plot: "Thái Công Vọng (Taikoubou) được giao sứ mệnh Phong Thần, tập hợp các đạo sĩ và thần khí tiêu diệt Đát Kỷ và triều đình Trụ Vương tàn bạo.", studio: "C-Station", genres: ["Animation", "Action", "Adventure", "Fantasy", "Mythology"] },
      { id: "79284", title: "Houshin Engi (1999)", vn: "Phong Thần Bảng (1999)", year: "1999", seasons: "1 Season", files: 27, qual: "480p DVD", qualClass: "480p", type: "anime", poster: "/api/poster?tvdb=79284", backdrop: "", path: "gdrive:Phim/TV Shows/Houshin Engi (1999) {tvdb-79284} [tvdbid-79284]", seasonList: ["Season 01 (26/26 tập 480p DVD)"], plot: "Bản anime kinh điển năm 1999 chuyển thể từ bộ truyện tranh Phong Thần Diễn Nghĩa của tác giả Ryu Fujisaki.", studio: "Studio Deen", genres: ["Animation", "Action", "Fantasy", "Shounen"] },
      { id: "299770", title: "Young Black Jack (2015)", vn: "Bác Sĩ Black Jack Thời Trẻ", year: "2015", seasons: "1 Season", files: 25, qual: "1080p BDRip", qualClass: "1080p", type: "anime", poster: "/api/poster?tvdb=299770", backdrop: "https://media.kitsu.app/anime/cover_images/10914/large.jpg", path: "gdrive:Phim/TV Shows/Young Black Jack (2015) {tvdb-299770} [tvdbid-299770]", seasonList: ["Season 01 (12/12 tập 1080p BDRip)"], plot: "Những năm 1960 trong bối cảnh phong trào sinh viên Nhật Bản và chiến tranh Việt Nam, chàng sinh viên y khoa Hazama Kuroo (Black Jack) bắt đầu thể hiện tài năng phẫu thuật xuất chúng.", studio: "Tezuka Productions", genres: ["Animation", "Drama", "Medical", "Psychological"] }
    ];

    let currentPlexFilter = "all";

    function renderPlexGrid(items) {
      loadLibraryStats();
      const grid = document.getElementById('plex-poster-grid');
      grid.innerHTML = items.map(s => `
        <div onclick="openPlexDetail('${s.id}')" class="group cursor-pointer rounded-2xl bg-zinc-900/80 border border-zinc-800 hover:border-[#e5a00d]/80 transition-all duration-300 hover:shadow-2xl hover:shadow-[#e5a00d]/15 flex flex-col overflow-hidden">
          <!-- Poster Frame with Real Image (2:3 Aspect Ratio) -->
          <div class="aspect-[2/3] w-full bg-zinc-950 relative overflow-hidden">
            <img src="${s.poster}&title=${encodeURIComponent(s.title)}" alt="${s.title}" class="w-full h-full object-cover group-hover:scale-105 transition duration-500" loading="lazy" onerror="this.onerror=null; this.src='/api/poster?title=' + encodeURIComponent(s.title);">
            
            <!-- Dark Gradient Vignette Overlay -->
            <div class="absolute inset-0 bg-gradient-to-t from-zinc-950 via-transparent to-black/40"></div>

            <!-- Top Badges -->
            <div class="absolute top-2 left-2 right-2 flex justify-between items-start gap-1">
              <span class="px-2 py-0.5 rounded bg-black/80 backdrop-blur text-[10px] font-bold text-white border border-white/15">${s.year}</span>
              <span class="px-2 py-0.5 rounded bg-[#e5a00d] text-[10px] font-black text-black shadow-md">${s.qual}</span>
            </div>

            <!-- Bottom Files & Seasons Pills -->
            <div class="absolute bottom-2 left-2 right-2 flex justify-between items-center text-[10px] text-zinc-200 bg-black/80 backdrop-blur px-2.5 py-1 rounded-lg border border-white/15">
              <span class="font-medium">${s.seasons}</span>
              <span class="font-bold text-[#e5a00d]">${s.files} files</span>
            </div>
          </div>

          <!-- Card Body Info -->
          <div class="p-3 space-y-1 flex-1 flex flex-col justify-between bg-zinc-900/60">
            <div>
              <div class="font-bold text-xs text-white truncate group-hover:text-[#e5a00d] transition" title="${s.title}">${s.title}</div>
              <div class="text-[11px] text-zinc-400 truncate" title="${s.vn}">${s.vn}</div>
            </div>
            <div class="text-[9px] font-mono text-zinc-500 truncate pt-1">tvdb-${s.id}</div>
          </div>
        </div>
      `).join('');
    }

    function filterPlexShows() {
      const q = (document.getElementById('plex-search')?.value || '').toLowerCase();
      let filtered = plexShowsData.filter(s => s.title.toLowerCase().includes(q) || s.vn.toLowerCase().includes(q) || s.id.includes(q));
      
      // Filter by selected Storage Hub (gdrive vs nas)
      if (currentSelectedStorage === "nas") {
        // Shows available on NAS Storage
        filtered = filtered.filter(s => s.id === "320122" || s.id === "78864");
      }

      if (currentPlexFilter !== "all") {
        if (currentPlexFilter === "1080p") filtered = filtered.filter(s => s.qual.includes("1080p"));
        else if (currentPlexFilter === "480p") filtered = filtered.filter(s => s.qual.includes("480p"));
        else if (currentPlexFilter === "anime") filtered = filtered.filter(s => s.type === "anime");
        else if (currentPlexFilter === "live") filtered = filtered.filter(s => s.type === "live");
      }

      const subLabel = document.getElementById('active-storage-subtitle');
      if (subLabel) {
        subLabel.innerText = `• ${filtered.length} Phim hiển thị`;
      }

      renderPlexGrid(filtered);
    }

    
    let currentSelectedStorage = "gdrive";

    function selectStorageSource(source) {
      currentSelectedStorage = source;
      const gdriveCard = document.getElementById('storage-card-gdrive');
      const nasCard = document.getElementById('storage-card-nas');
      const gdriveIndicator = document.getElementById('gdrive-active-indicator');
      const nasIndicator = document.getElementById('nas-active-indicator');
      const activeIcon = document.getElementById('active-storage-icon');
      const activeTitle = document.getElementById('active-storage-title');

      if (source === 'gdrive') {
        gdriveCard.className = "p-5 rounded-3xl bg-emerald-950/30 border-2 border-emerald-500 ring-4 ring-emerald-500/10 shadow-xl cursor-pointer transition relative overflow-hidden group hover:scale-[1.01]";
        nasCard.className = "p-5 rounded-3xl bg-zinc-900/60 border border-zinc-800/80 hover:border-amber-500/50 shadow-md cursor-pointer transition relative overflow-hidden group hover:scale-[1.01] opacity-75 hover:opacity-100";
        
        gdriveIndicator.className = "px-2.5 py-1 rounded-full bg-emerald-500 text-black font-bold text-[10px] shadow-md flex items-center gap-1";
        gdriveIndicator.innerHTML = "<span>✓</span> Đang Xem";

        nasIndicator.className = "px-2.5 py-1 rounded-full bg-zinc-800 text-zinc-400 font-medium text-[10px] flex items-center gap-1";
        nasIndicator.innerHTML = "Bấm để xem";

        if (activeIcon) activeIcon.innerText = "☁️";
        if (activeTitle) activeTitle.innerText = "Đang Duyệt: Google Drive (gdrive:Phim)";
      } else if (source === 'nas') {
        nasCard.className = "p-5 rounded-3xl bg-amber-950/30 border-2 border-amber-500 ring-4 ring-amber-500/10 shadow-xl cursor-pointer transition relative overflow-hidden group hover:scale-[1.01]";
        gdriveCard.className = "p-5 rounded-3xl bg-zinc-900/60 border border-zinc-800/80 hover:border-emerald-500/50 shadow-md cursor-pointer transition relative overflow-hidden group hover:scale-[1.01] opacity-75 hover:opacity-100";

        nasIndicator.className = "px-2.5 py-1 rounded-full bg-amber-500 text-black font-bold text-[10px] shadow-md flex items-center gap-1";
        nasIndicator.innerHTML = "<span>✓</span> Đang Xem";

        gdriveIndicator.className = "px-2.5 py-1 rounded-full bg-zinc-800 text-zinc-400 font-medium text-[10px] flex items-center gap-1";
        gdriveIndicator.innerHTML = "Bấm để xem";

        if (activeIcon) activeIcon.innerText = "🖥️";
        if (activeTitle) activeTitle.innerText = "Đang Duyệt: NAS Storage (/srv/mergerfs/MainPool/Phim)";
      }

      filterPlexShows();
    }

    function filterPlexTag(tag) {
      currentPlexFilter = tag;
      document.querySelectorAll('.plex-filter-btn').forEach(btn => {
        btn.className = "plex-filter-btn px-3 py-1.5 rounded-lg bg-zinc-900 border border-zinc-800 text-zinc-400 hover:text-white transition";
      });
      document.getElementById(`pfilter-${tag}`).className = "plex-filter-btn px-3 py-1.5 rounded-lg bg-[#e5a00d] text-black font-bold transition shadow";
      filterPlexShows();
    }

    let currentActiveShow = null;

    function openPlexDetail(id) {
      const s = plexShowsData.find(x => x.id === id);
      if (!s) return;
      currentActiveShow = s;

      document.getElementById('plex-view-library').classList.add('hidden');
      const libHeader = document.getElementById('gdrive-header-library');
      if (libHeader) libHeader.classList.add('hidden');
      
      document.getElementById('plex-view-show').classList.remove('hidden');
      const showHeader = document.getElementById('gdrive-header-show');
      if (showHeader) showHeader.classList.remove('hidden');

      // Update Top Show Header & Inline Name
      const headerTitle = document.getElementById('header-show-title');
      const headerBadge = document.getElementById('header-show-badge');
      const headerSub = document.getElementById('header-show-subtitle');
      const inlineName = document.getElementById('inline-show-name');
      if (headerTitle) headerTitle.innerText = s.title;
      if (headerBadge) headerBadge.innerText = s.qual || 'Drive Ready';
      if (headerSub) headerSub.innerText = `${s.vn} • ${s.seasons} • ${s.files} files`;
      if (inlineName) inlineName.innerText = `${s.title} (${s.vn})`;

      // Populate Hero Banner
      document.getElementById('show-hero-title').innerText = s.title;
      document.getElementById('show-hero-vn').innerText = s.vn;
      document.getElementById('show-hero-year').innerText = s.year;
      document.getElementById('show-hero-tvdb').innerText = `{tvdb-${s.id}}`;
      document.getElementById('show-hero-plot').innerText = s.plot || `${s.vn} là một tác phẩm kinh điển. Đã chuẩn hóa toàn bộ file theo quy chuẩn {Show Name} - S{XX}E{YY} - [{Quality}]. Toàn bộ dữ liệu nằm an toàn trên Google Drive.`;
      document.getElementById('show-files-count').innerText = `${s.seasons} • ${s.files} files`;

      // Set Backdrop & Poster Image
      const backdrop = document.getElementById('show-backdrop-bg');
      if (backdrop) {
        backdrop.style.backgroundImage = `url('${s.backdrop || s.poster || `/api/poster?tvdb=${s.id}`}&title=${encodeURIComponent(s.title)}')`;
      }

      const posterBox = document.getElementById('show-poster-box');
      if (posterBox) {
        posterBox.innerHTML = `
          <img src="${s.poster}&title=${encodeURIComponent(s.title)}" alt="${s.title}" class="w-full h-full object-cover" onerror="this.onerror=null; this.src='/api/poster?title=' + encodeURIComponent(s.title);">
          <span class="absolute bottom-2 px-2 py-0.5 rounded bg-[#e5a00d] text-black font-black text-[10px] shadow-md">${s.qual}</span>
        `;
      }

      // Build Season Tabs
      const tabsContainer = document.getElementById('show-season-tabs');
      tabsContainer.innerHTML = s.seasonList.map((stext, idx) => {
        const sfolder = stext.split(" (")[0].trim();
        const isFirst = idx === 0;
        return `
          <button onclick="selectSeason('${s.path.split('/')[2]}', '${sfolder}', this)" class="season-pill-btn px-4 py-2 rounded-xl text-xs font-semibold whitespace-nowrap transition ${isFirst ? 'bg-[#e5a00d] text-black shadow-md shadow-amber-500/20' : 'bg-zinc-900 hover:bg-zinc-800 text-zinc-300 border border-zinc-800'}">
            📁 ${stext}
          </button>
        `;
      }).join('');

      // Auto-load first season episodes
      if (s.seasonList.length > 0) {
        const firstSeason = s.seasonList[0].split(" (")[0].trim();
        const showFolder = s.path.split('/')[2];
        loadSeasonEpisodes(showFolder, firstSeason);
      }
    }

    function backToPlexLibrary() {
      document.getElementById('plex-view-show').classList.add('hidden');
      const showHeader = document.getElementById('gdrive-header-show');
      if (showHeader) showHeader.classList.add('hidden');

      document.getElementById('plex-view-library').classList.remove('hidden');
      const libHeader = document.getElementById('gdrive-header-library');
      if (libHeader) libHeader.classList.remove('hidden');
    }

    function selectSeason(showFolder, seasonName, btn) {
      document.querySelectorAll('.season-pill-btn').forEach(b => {
        b.className = "season-pill-btn px-4 py-2 rounded-xl text-xs font-semibold whitespace-nowrap transition bg-zinc-900 hover:bg-zinc-800 text-zinc-300 border border-zinc-800";
      });
      btn.className = "season-pill-btn px-4 py-2 rounded-xl text-xs font-semibold whitespace-nowrap transition bg-[#e5a00d] text-black shadow-md shadow-amber-500/20";
      loadSeasonEpisodes(showFolder, seasonName);
    }

    // In-memory & Persistent Season Files Cache
    let seasonFilesMemoryCache = {};
    try {
      seasonFilesMemoryCache = JSON.parse(localStorage.getItem('gdrive_season_cache_v1') || '{}');
    } catch (e) {}

    function renderEpisodesList(container, files, showFolder, seasonName) {
      if (!files || files.length === 0) {
        container.innerHTML = `
          <div class="p-6 rounded-2xl bg-zinc-950 border border-zinc-800 text-center text-xs text-zinc-500">
            Chưa có file nào trong thư mục ${seasonName}.
          </div>
        `;
        return;
      }

      // Separate videos and subtitles
      const videoFiles = files.filter(f => f.endsWith('.mkv') || f.endsWith('.mp4') || f.endsWith('.avi'));
      const subFiles = files.filter(f => f.endsWith('.ass') || f.endsWith('.srt'));
      const displayItems = videoFiles.length > 0 ? videoFiles : files;

      container.innerHTML = displayItems.map((fname, idx) => {
        const hasVietsub = subFiles.some(s => s.includes(fname.substring(0, 15)));
        
        let qualTag = "1080p";
        if (fname.includes("1080p")) qualTag = "1080p BDRip";
        else if (fname.includes("480p")) qualTag = "480p DVD";
        else if (fname.includes("720p")) qualTag = "720p HDTV";

        const downloadUrl = `/api/download?show=${encodeURIComponent(showFolder)}&season=${encodeURIComponent(seasonName)}&file=${encodeURIComponent(fname)}`;
        const streamUrl = `/api/stream?show=${encodeURIComponent(showFolder)}&season=${encodeURIComponent(seasonName)}&file=${encodeURIComponent(fname)}`;

        return `
          <div class="p-3.5 rounded-2xl bg-zinc-950 border border-zinc-800/80 hover:border-zinc-700 transition flex flex-col md:flex-row justify-between items-start md:items-center gap-3">
            <div class="flex items-center gap-3 min-w-0 flex-1">
              <div class="w-10 h-10 rounded-xl bg-zinc-900 border border-zinc-800 flex items-center justify-center font-mono font-bold text-xs text-amber-400 shrink-0">
                ${idx + 1 < 10 ? '0' + (idx + 1) : idx + 1}
              </div>
              <div class="min-w-0 flex-1">
                <div class="font-semibold text-xs text-white truncate" title="${fname}">${fname}</div>
                <div class="flex items-center gap-2 text-[10px] text-zinc-400 mt-1">
                  <span class="px-1.5 py-0.5 rounded bg-zinc-900 border border-zinc-800 font-mono text-zinc-300">${qualTag}</span>
                  ${hasVietsub ? '<span class="px-1.5 py-0.5 rounded bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 font-semibold">🇻🇳 Vietsub ASS</span>' : ''}
                  <span class="text-zinc-500">• Direct Stream Ready</span>
                </div>
              </div>
            </div>

            <!-- Action Buttons: Play | Download | Copy Link -->
            <div class="flex items-center gap-2 shrink-0 self-end md:self-center">
              <button onclick="playEpisode('${encodeURIComponent(showFolder)}', '${encodeURIComponent(seasonName)}', '${encodeURIComponent(fname)}')" class="px-3 py-1.5 bg-amber-500/10 hover:bg-[#e5a00d] text-amber-400 hover:text-black border border-amber-500/30 text-xs font-semibold rounded-xl transition flex items-center gap-1.5 shadow-sm shadow-amber-500/10">
                <span>▶️</span> Phát
              </button>
              
              <a href="${downloadUrl}" download="${fname}" class="px-3 py-1.5 bg-blue-500/10 hover:bg-blue-600 text-blue-400 hover:text-white border border-blue-500/30 text-xs font-semibold rounded-xl transition flex items-center gap-1.5">
                <span>⬇️</span> Tải
              </a>

              <button onclick="copyEpisodeLink('${encodeURIComponent(showFolder)}', '${encodeURIComponent(seasonName)}', '${encodeURIComponent(fname)}')" class="px-3 py-1.5 bg-zinc-900 hover:bg-zinc-800 text-zinc-300 hover:text-white border border-zinc-800 text-xs font-medium rounded-xl transition flex items-center gap-1.5" title="Copy link xem & tải trực tiếp">
                <span>📋</span> Copy Link
              </button>
            </div>
          </div>
        `;
      }).join('');
    }

    let art = null;

    async function loadSeasonEpisodes(showFolder, seasonName, forceRefresh = false) {
      currentActiveSeason = { showFolder, seasonName };
      const container = document.getElementById('show-episodes-container');
      const cacheKey = `${showFolder}/${seasonName}`;

      // 1. If cached and not forced, render from RAM/LocalStorage with ZERO server hits!
      if (!forceRefresh && seasonFilesMemoryCache[cacheKey]) {
        renderEpisodesList(container, seasonFilesMemoryCache[cacheKey], showFolder, seasonName);
        return; // ZERO network request!
      }

      // 2. Only if not in cache or forced refresh, fetch from server
      container.innerHTML = `
        <div class="p-6 rounded-2xl bg-zinc-950 border border-zinc-800 text-center text-xs text-zinc-400 animate-pulse">
          ⚡ Đang đồng bộ danh mục tập từ Google Drive...
        </div>
      `;

      try {
        const url = `/api/gdrive/season_files?show=${encodeURIComponent(showFolder)}&season=${encodeURIComponent(seasonName)}${forceRefresh ? '&refresh=1' : ''}`;
        const res = await fetch(url);
        const data = await res.json();
        const freshFiles = data.files || [];

        // Save to in-memory and persistent localStorage cache
        seasonFilesMemoryCache[cacheKey] = freshFiles;
        try {
          localStorage.setItem('gdrive_season_cache_v1', JSON.stringify(seasonFilesMemoryCache));
        } catch (e) {}

        renderEpisodesList(container, freshFiles, showFolder, seasonName);
      } catch (e) {
        if (seasonFilesMemoryCache[cacheKey]) {
          renderEpisodesList(container, seasonFilesMemoryCache[cacheKey], showFolder, seasonName);
        } else {
          container.innerHTML = `<div class="p-4 rounded-xl bg-red-900/20 text-red-400 text-xs">Lỗi tải danh sách: ${e}</div>`;
        }
      }
    }

export {
  plexShowsData,
  currentPlexFilter,
  currentSelectedStorage,
  currentActiveShow,
  seasonFilesMemoryCache,
  renderPlexGrid,
  filterPlexShows,
  selectStorageSource,
  filterPlexTag,
  openPlexDetail,
  backToPlexLibrary,
  selectSeason,
  renderEpisodesList,
  loadSeasonEpisodes
};
