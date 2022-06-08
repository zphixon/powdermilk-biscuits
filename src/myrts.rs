#![allow(non_snake_case, non_camel_case_types, unused_imports)]

use std::sync::mpsc::{self, Receiver, Sender};

use windows::{
    core::{implement, Result, GUID, HRESULT},
    Win32::{
        Foundation::{HANDLE_PTR, HWND, LPARAM, LRESULT, POINT, WPARAM},
        Graphics::Gdi::ValidateRect,
        System::{
            Com::{
                CLSIDFromString, CoCreateInstance, CoInitialize, CoInitializeEx, CoUninitialize,
                CLSCTX_ALL, COINIT_APARTMENTTHREADED, COINIT_MULTITHREADED,
            },
            LibraryLoader::GetModuleHandleA,
        },
        UI::{
            Input::KeyboardAndMouse::{VIRTUAL_KEY, VK_ESCAPE},
            TabletPC::{
                IInkTablet, IRealTimeStylus, IStylusAsyncPlugin, IStylusAsyncPlugin_Impl,
                IStylusPlugin, IStylusPlugin_Impl, RTSDI_AllData, RealTimeStylus,
                RealTimeStylusDataInterest, StylusInfo, GUID_PACKETPROPERTY_GUID_NORMAL_PRESSURE,
                GUID_PACKETPROPERTY_GUID_PACKET_STATUS, GUID_PACKETPROPERTY_GUID_X,
                GUID_PACKETPROPERTY_GUID_Y, SYSTEM_EVENT_DATA,
            },
            WindowsAndMessaging::{
                CreateWindowExA, DefWindowProcA, DispatchMessageA, GetMessageA, LoadCursorW,
                PostQuitMessage, RegisterClassA, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, IDC_ARROW,
                MSG, WM_DESTROY, WM_KEYDOWN, WM_PAINT, WNDCLASSA, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
            },
        },
    },
};

#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Invert,
    Down(u64, u64),
    Up(u64, u64),
    Move(u64, u64),
}

#[implement(IStylusAsyncPlugin)]
#[derive(Debug)]
pub struct RtsHandler {
    sender: Sender<Msg>,
}

impl RtsHandler {
    pub fn new(hwnd: HWND) -> Result<Receiver<Msg>> {
        unsafe {
            let (sender, receiver) = mpsc::channel();

            std::thread::spawn(move || {
                CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED).unwrap();

                let rts: IRealTimeStylus =
                    Some(CoCreateInstance(&RealTimeStylus as *const _, None, CLSCTX_ALL).unwrap())
                        .unwrap();

                rts.SetHWND(HANDLE_PTR(hwnd.0.try_into().unwrap())).unwrap();

                rts.SetDesiredPacketDescription(&[
                    GUID_PACKETPROPERTY_GUID_X,
                    GUID_PACKETPROPERTY_GUID_Y,
                    GUID_PACKETPROPERTY_GUID_NORMAL_PRESSURE,
                    GUID_PACKETPROPERTY_GUID_PACKET_STATUS,
                ])
                .unwrap();

                let plugin: IStylusAsyncPlugin = RtsHandler { sender }.into();
                rts.AddStylusAsyncPlugin(0, plugin).unwrap();

                rts.SetEnabled(true).unwrap();
                Box::leak(Box::new(rts));
            });

            Ok(receiver)
        }
    }
}

impl IStylusAsyncPlugin_Impl for RtsHandler {}

impl IStylusPlugin_Impl for RtsHandler {
    fn Packets(
        &self,
        _pirtssrc: &Option<IRealTimeStylus>,
        _pstylusinfo: *const StylusInfo,
        _cpktcount: u32,
        _cpktbufflength: u32,
        _ppackets: *const i32,
        _pcinoutpkts: *mut u32,
        _ppinoutpkts: *mut *mut i32,
    ) -> Result<()> {
        //println!("{}", std::process::id());
        //println!("pirtssrc={pirtssrc:?}");
        //println!("pstylusinfo={pstylusinfo:?}");
        //println!("cpktcount={cpktcount}");
        //println!("cpktbufflength={cpktbufflength}");
        //println!("ppackets={ppackets:?}");
        //println!("pcinoutpkts={pcinoutpkts:?}");
        //println!("ppinoutpkts={ppinoutpkts:?}");
        self.sender.send(Msg::Move(32, 32)).unwrap();
        println!("fuck!");
        Ok(())
    }

    fn DataInterest(&self) -> Result<RealTimeStylusDataInterest> {
        //println!("wow!");
        //println!("pid={}", std::process::id());
        Ok(RTSDI_AllData)
    }

    // methods we don't care about yet {{{
    fn RealTimeStylusEnabled(
        &self,
        _pirtssrc: &Option<IRealTimeStylus>,
        _ctcidcount: u32,
        _ptcids: *const u32,
    ) -> Result<()> {
        println!("nice?");
        Ok(())
    }

    fn RealTimeStylusDisabled(
        &self,
        _pirtssrc: &Option<IRealTimeStylus>,
        _ctcidcount: u32,
        _ptcids: *const u32,
    ) -> Result<()> {
        Ok(())
    }

    fn StylusInRange(
        &self,
        _pirtssrc: &Option<IRealTimeStylus>,
        _tcid: u32,
        _sid: u32,
    ) -> Result<()> {
        Ok(())
    }

    fn StylusOutOfRange(
        &self,
        _pirtssrc: &Option<IRealTimeStylus>,
        _tcid: u32,
        _sid: u32,
    ) -> Result<()> {
        Ok(())
    }

    fn StylusDown(
        &self,
        _pirtssrc: &Option<IRealTimeStylus>,
        _pstylusinfo: *const StylusInfo,
        _cpropcountperpkt: u32,
        _ppacket: *const i32,
        _ppinoutpkt: *mut *mut i32,
    ) -> Result<()> {
        Ok(())
    }

    fn StylusUp(
        &self,
        _pirtssrc: &Option<IRealTimeStylus>,
        _pstylusinfo: *const StylusInfo,
        _cpropcountperpkt: u32,
        _ppacket: *const i32,
        _ppinoutpkt: *mut *mut i32,
    ) -> Result<()> {
        Ok(())
    }

    fn StylusButtonDown(
        &self,
        _pirtssrc: &Option<IRealTimeStylus>,
        _sid: u32,
        _pguidstylusbutton: *const GUID,
        _pstyluspos: *mut POINT,
    ) -> Result<()> {
        Ok(())
    }

    fn StylusButtonUp(
        &self,
        _pirtssrc: &Option<IRealTimeStylus>,
        _sid: u32,
        _pguidstylusbutton: *const GUID,
        _pstyluspos: *mut POINT,
    ) -> Result<()> {
        Ok(())
    }

    fn InAirPackets(
        &self,
        _pirtssrc: &Option<IRealTimeStylus>,
        _pstylusinfo: *const StylusInfo,
        _cpktcount: u32,
        _cpktbufflength: u32,
        _ppackets: *const i32,
        _pcinoutpkts: *mut u32,
        _ppinoutpkts: *mut *mut i32,
    ) -> Result<()> {
        Ok(())
    }

    fn CustomStylusDataAdded(
        &self,
        _pirtssrc: &Option<IRealTimeStylus>,
        _pguidid: *const GUID,
        _cbdata: u32,
        _pbdata: *const u8,
    ) -> Result<()> {
        Ok(())
    }

    fn SystemEvent(
        &self,
        _pirtssrc: &Option<IRealTimeStylus>,
        _tcid: u32,
        _sid: u32,
        _event: u16,
        _eventdata: &SYSTEM_EVENT_DATA,
    ) -> Result<()> {
        Ok(())
    }

    fn TabletAdded(
        &self,
        _pirtssrc: &Option<IRealTimeStylus>,
        _pitablet: &Option<IInkTablet>,
    ) -> Result<()> {
        Ok(())
    }

    fn TabletRemoved(&self, _pirtssrc: &Option<IRealTimeStylus>, _itabletindex: i32) -> Result<()> {
        Ok(())
    }

    fn Error(
        &self,
        _pirtssrc: &Option<IRealTimeStylus>,
        _piplugin: &Option<IStylusPlugin>,
        _datainterest: RealTimeStylusDataInterest,
        _hrerrorcode: HRESULT,
        _lptrkey: *mut isize,
    ) -> Result<()> {
        Ok(())
    }

    fn UpdateMapping(&self, _pirtssrc: &Option<IRealTimeStylus>) -> Result<()> {
        Ok(())
    }
    // }}}
}

//extern "system" fn wndproc(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
//    unsafe {
//        match message {
//            WM_PAINT => {
//                ValidateRect(window, std::ptr::null());
//                LRESULT(0)
//            }
//
//            WM_DESTROY => {
//                println!("bye");
//                PostQuitMessage(0);
//                LRESULT(0)
//            }
//
//            WM_KEYDOWN => {
//                if VIRTUAL_KEY(wparam.0 as u16) == VK_ESCAPE {
//                    PostQuitMessage(0);
//                }
//
//                LRESULT(0)
//            }
//
//            _ => DefWindowProcA(window, message, wparam, lparam),
//        }
//    }
//}
//
//pub fn do_stuff() -> Result<()> {
//    unsafe {
//        println!("pid={}", std::process::id());
//        let class_name = "class name heehee\0";
//
//        let instance = GetModuleHandleA(None)?;
//        assert_ne!(instance.0, 0);
//
//        let wc = WNDCLASSA {
//            hCursor: LoadCursorW(None, IDC_ARROW)?,
//            hInstance: instance,
//            lpszClassName: windows::core::PCSTR(class_name.as_ptr()),
//            style: CS_HREDRAW | CS_VREDRAW,
//            lpfnWndProc: Some(wndproc),
//            ..Default::default()
//        };
//
//        let atom = RegisterClassA(&wc);
//        assert_ne!(atom, 0);
//
//        let hwnd = CreateWindowExA(
//            Default::default(),
//            class_name,
//            "wowie",
//            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
//            CW_USEDEFAULT,
//            CW_USEDEFAULT,
//            CW_USEDEFAULT,
//            CW_USEDEFAULT,
//            None,
//            None,
//            instance,
//            std::ptr::null(),
//        );
//
//        let (_rts, _reciever) = RtsHandler::new(hwnd, true)?;
//
//        let mut message = MSG::default();
//        while GetMessageA(&mut message, HWND(0), 0, 0).into() {
//            DispatchMessageA(&message);
//        }
//
//        CoUninitialize();
//        //todo!();
//
//        Ok(())
//    }
//}
